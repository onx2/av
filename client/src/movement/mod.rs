mod movement_state;
mod transform;

use crate::{ActorEntity, secondary_stats::SecondaryStats};
use bevy::prelude::*;
use movement_state::*;
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
    time: Res<Time<Fixed>>,
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
    mut query: Query<(&mut SimTransform, &SimMovementState, &SecondaryStats), With<ActorEntity>>,
) {
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
