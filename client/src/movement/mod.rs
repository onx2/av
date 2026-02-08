pub mod movement_state;
pub mod transform;

use crate::module_bindings::MoveIntentData;
use crate::{ActorEntity, secondary_stats::SecondaryStats, world::ClientStaticQueryWorld};
use bevy::prelude::*;
use movement_state::*;
use nalgebra::{Isometry3, Translation3, UnitQuaternion, Vector2, Vector3};
use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    prelude::{Capsule, QueryFilter},
};
use shared::{advance_vertical_velocity, get_desired_delta, yaw_from_xz};
use transform::*;

#[derive(Resource, Debug, Default)]
pub struct ClientIntentSeq(pub u32);

#[derive(Component, Debug, Default)]
pub struct LastAckIntentSeq(pub u32);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ClientIntentSeq>();
    app.add_plugins((transform::plugin, movement_state::plugin));
    app.add_systems(FixedUpdate, (reconcile, predict).chain());
    app.add_systems(Update, interpolate);
}

fn reconcile(
    mut query: Query<
        (
            &NetTransform,
            &NetMovementState,
            &mut LastAckIntentSeq,
            &mut SimTransform,
            &mut SimMovementState,
        ),
        With<ActorEntity>,
    >,
) {
    query.par_iter_mut().for_each(
        |(
            net_transform,
            net_movement_state,
            mut last_ack_intent_seq,
            mut sim_transform,
            mut sim_movement_state,
        )| {
            if net_transform.client_intent_seq > last_ack_intent_seq.0 {
                sim_transform.translation = net_transform.translation;
                sim_movement_state.move_intent = net_movement_state.move_intent.clone();
                sim_movement_state.cell_id = net_movement_state.cell_id;
                sim_movement_state.should_move = net_movement_state.should_move;
                sim_movement_state.vertical_velocity = net_movement_state.vertical_velocity;
                last_ack_intent_seq.0 = net_transform.client_intent_seq;
            }
        },
    );
}

fn predict(
    time: Res<Time<Fixed>>,
    query_world: Res<ClientStaticQueryWorld>,
    mut query: Query<
        (&mut SimTransform, &mut SimMovementState, &SecondaryStats),
        With<ActorEntity>,
    >,
) {
    let dt = time.delta_secs();

    let Some(query_world) = query_world.query_world.as_ref() else {
        return;
    };

    let kcc = KinematicCharacterController {
        autostep: Some(CharacterAutostep {
            include_dynamic_bodies: false,
            max_height: CharacterLength::Relative(0.4),
            ..CharacterAutostep::default()
        }),
        offset: CharacterLength::Relative(0.025),
        ..KinematicCharacterController::default()
    };

    let query_pipeline = query_world.as_query_pipeline(QueryFilter::only_fixed());

    // Temporary static capsule values (until replicated).
    const CAPSULE_RADIUS: f32 = 0.3;
    const CAPSULE_HALF_HEIGHT: f32 = 0.9;
    let capsule = Capsule::new_y(CAPSULE_HALF_HEIGHT, CAPSULE_RADIUS);

    query.iter_mut().for_each(
        |(mut sim_transform, mut sim_movement_state, secondary_stats)| {
            if !sim_movement_state.should_move && sim_movement_state.vertical_velocity >= 0 {
                return;
            }

            if sim_movement_state.vertical_velocity < 0 {
                sim_movement_state.vertical_velocity =
                    advance_vertical_velocity(sim_movement_state.vertical_velocity, dt);
            }

            let current_planar = sim_transform.translation.xz();
            let target_planar = match &sim_movement_state.move_intent {
                MoveIntentData::Point(point) => Vec2::new(point.x, point.z),
                MoveIntentData::Path(path) => path
                    .first()
                    .map(|p| Vec2::new(p.x, p.z))
                    .unwrap_or(current_planar),
                _ => current_planar,
            };

            let movement_speed_mps = secondary_stats.movement_speed;
            let direction = (target_planar - current_planar)
                .try_normalize()
                .unwrap_or_default();

            let yaw = yaw_from_xz(Vector2::new(direction.x, direction.y)).unwrap_or(0.0);

            let desired_delta = get_desired_delta(
                Vector2::new(current_planar.x, current_planar.y),
                Vector2::new(target_planar.x, target_planar.y),
                movement_speed_mps,
                sim_movement_state.vertical_velocity,
                dt,
            );

            let iso: Isometry3<f32> = Isometry3::from_parts(
                Translation3::new(
                    sim_transform.translation.x,
                    sim_transform.translation.y,
                    sim_transform.translation.z,
                ),
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw),
            );

            let correction = kcc.move_shape(
                dt,
                &query_pipeline,
                &capsule,
                &iso,
                desired_delta.into(),
                |_| {},
            );

            sim_transform.translation.x += correction.translation.x;
            sim_transform.translation.y += correction.translation.y;
            sim_transform.translation.z += correction.translation.z;

            if correction.grounded {
                sim_movement_state.vertical_velocity = 0;
            } else if sim_movement_state.vertical_velocity == 0 {
                sim_movement_state.vertical_velocity = -1;
            }
        },
    );
}

fn interpolate(time: Res<Time>, mut transform_query: Query<(&mut Transform, &SimTransform)>) {
    let dt = time.delta_secs();
    transform_query
        .par_iter_mut()
        .for_each(|(mut render_transform, sim_transform)| {
            // Smoothly nudge this value towards the target at a given decay rate. The decay_rate parameter controls how fast the distance between self and target decays relative to the units of delta; the intended usage is for decay_rate to generally remain fixed, while delta is something like delta_time from an updating system. This produces a smooth following of the target that is independent of framerate.
            // More specifically, when this is called repeatedly, the result is that the distance between self and a fixed target attenuates exponentially, with the rate of this exponential decay given by decay_rate.
            // For example, at decay_rate = 0.0, this has no effect. At decay_rate = f32::INFINITY, self immediately snaps to target. In general, higher rates mean that self moves more quickly towards target.
            render_transform
                .translation
                .smooth_nudge(&sim_transform.translation, 12.0, dt);
        });
}
