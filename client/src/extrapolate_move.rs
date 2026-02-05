use crate::module_bindings::MoveIntentData;
use crate::movement_state::MovementState;
use crate::secondary_stats::SecondaryStats;
use crate::world::ClientStaticQueryWorld;
use crate::{
    ActorEntity, RemoteActor,
    transform::{LastNetRecvTime, NetTransform, PredictedTransform},
};
use bevy::prelude::*;
use nalgebra::Vector2;
use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    na::{Isometry3, Translation3, UnitQuaternion, Vector3},
    prelude::{Capsule, QueryFilter},
};
use shared::{advance_vertical_velocity, get_desired_delta, yaw_from_xz};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, extrapolate_move);
}

fn extrapolate_move(
    fixed_time: Res<Time<Fixed>>,
    time: Res<Time>,
    query_world: Res<ClientStaticQueryWorld>,
    mut query: Query<
        (
            &mut PredictedTransform,
            &NetTransform,
            &mut MovementState,
            &SecondaryStats,
            Option<&LastNetRecvTime>,
            Option<&RemoteActor>,
        ),
        With<ActorEntity>,
    >,
) {
    let dt = fixed_time.delta_secs();

    // Don’t extrapolate remote actors indefinitely if snapshots stall.
    const MAX_REMOTE_EXTRAPOLATE_SECS: f64 = 0.25;

    // Temporary static capsule values (until replicated).
    const CAPSULE_RADIUS: f32 = 0.3;
    const CAPSULE_HALF_HEIGHT: f32 = 0.9;

    let Some(query_world) = query_world.query_world.as_ref() else {
        // No collision world yet; can't run KCC.
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
    let capsule = Capsule::new_y(CAPSULE_HALF_HEIGHT, CAPSULE_RADIUS);

    for (mut predicted, net, mut movement_state, secondary_stats, last_net_recv, remote_tag) in
        query.iter_mut()
    {
        if !movement_state.should_move {
            continue;
        }

        // If this is a remote actor and we haven't received a net snapshot recently,
        // freeze (do not extrapolate further) to avoid runaway drift.
        if remote_tag.is_some() {
            let Some(last_net_recv) = last_net_recv else {
                continue;
            };
            let age = time.elapsed_secs_f64() - last_net_recv.secs;
            if age > MAX_REMOTE_EXTRAPOLATE_SECS {
                continue;
            }
        }

        // Critical: always integrate from the latest authoritative pose.
        //
        // Without this, the simulation can "run away" between snapshots and then get hard-corrected
        // at the next server tick, producing visible snapping at 20 Hz. Resetting the base pose
        // each fixed tick makes KCC behave like "extrapolate a small step from the last snapshot".
        predicted.translation = net.translation;
        predicted.rotation = net.rotation;

        // Mirror server: if falling, advance vv first.
        if movement_state.vertical_velocity < 0 {
            movement_state.vertical_velocity =
                advance_vertical_velocity(movement_state.vertical_velocity, dt);
        }

        let current_planar = predicted.translation.xz();
        let target_planar = match &movement_state.move_intent {
            MoveIntentData::Point(point) => Vec2::new(point.x, point.z),
            _ => current_planar,
        };

        let movement_speed_mps = secondary_stats.movement_speed;
        let direction = (target_planar - current_planar)
            .try_normalize()
            .unwrap_or_default();

        if let Some(yaw) = yaw_from_xz(Vector2::new(direction.x, direction.y)) {
            predicted.rotation = Quat::from_rotation_y(yaw);
        }

        let desired_delta = get_desired_delta(
            Vector2::new(current_planar.x, current_planar.y),
            Vector2::new(target_planar.x, target_planar.y),
            movement_speed_mps,
            movement_state.vertical_velocity,
            dt,
        );

        // Match server behavior: KCC uses an isometry built from translation + yaw about Y.
        let (yaw, _, _) = predicted.rotation.to_euler(EulerRot::YXZ);

        let iso: Isometry3<f32> = Isometry3::from_parts(
            Translation3::new(
                predicted.translation.x,
                predicted.translation.y,
                predicted.translation.z,
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

        predicted.translation.x += correction.translation.x;
        predicted.translation.y += correction.translation.y;
        predicted.translation.z += correction.translation.z;

        // Ground truth for grounding comes from KCC.
        if correction.grounded {
            movement_state.vertical_velocity = 0;
        } else if movement_state.vertical_velocity == 0 {
            movement_state.vertical_velocity = -1;
        }
    }
}
