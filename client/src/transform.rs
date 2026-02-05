use crate::{
    LocalActor, RemoteActor,
    actor::{ActorEntityMapping, ensure_actor_entity},
    module_bindings::TransformRow,
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

/// Cached server transform data for an entity.
#[derive(Component, Debug)]
pub struct NetTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

/// The pose produced by client simulation (prediction for local, extrapolation for remote).
#[derive(Component, Debug)]
pub struct PredictedTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

/// A visual-only offset applied on top of `PredictedTransform` that decays over time.
///
/// This is the "Option B" correction path:
/// - When an authoritative server transform arrives for the local actor, compute the error between
///   the server pose and the predicted pose, and add it to this offset.
/// - Each frame/tick, exponentially decay this offset toward zero.
/// - Render pose = predicted pose + offset, which yields stable visuals without fighting simulation.
#[derive(Component, Debug, Default)]
pub struct PredictionErrorOffset {
    pub translation: Vec3,
    pub yaw: f32,
}

/// Timestamp (client time) for when we last received an authoritative transform snapshot.
///
/// This lives in the transform replication module so extrapolation systems can bound how far they
/// run past the last server snapshot for remote actors.
#[derive(Component, Debug, Default)]
pub struct LastNetRecvTime {
    pub secs: f64,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, (on_transform_inserted, on_transform_updated));
    // Presentation: apply predicted + error offset to actual render `Transform`.
    app.add_systems(Update, apply_predicted_to_render_transform);
}

fn on_transform_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<TransformRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
    time: Res<Time>,
) {
    for msg in msgs.read() {
        println!("on_transform_inserted: {:?}", msg.row.actor_id);
        // Ensure the owner entity exists regardless of message ordering.
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);

        // Use Commands to avoid timing issues with deferred spawns/components.
        let translation: Vec3 = msg.row.translation.clone().into();
        let rotation: Quat = Quat::from_rotation_y(msg.row.yaw);

        commands.entity(bevy_entity).insert((
            // Make visible now that we have a valid transform. TODO: this might not be necessary once assets for the character are used.
            Visibility::Inherited,
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
            NetTransform {
                translation,
                rotation,
            },
            PredictedTransform {
                translation,
                rotation,
            },
            PredictionErrorOffset::default(),
            LastNetRecvTime {
                secs: time.elapsed_secs_f64(),
            },
        ));
    }
}

fn on_transform_updated(
    time: Res<Time>,
    mut q: ParamSet<(
        // Local actor: keep predicting, and accumulate a decaying visual offset toward server authority.
        Query<
            (
                &mut NetTransform,
                &PredictedTransform,
                &mut PredictionErrorOffset,
                &mut LastNetRecvTime,
            ),
            With<LocalActor>,
        >,
        // Remote actors: re-seed the predicted pose from each server snapshot.
        Query<
            (
                &mut NetTransform,
                &mut PredictedTransform,
                &mut LastNetRecvTime,
            ),
            With<RemoteActor>,
        >,
    )>,
    mut msgs: ReadUpdateMessage<TransformRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };

        let new_net_translation: Vec3 = msg.new.translation.clone().into();
        let new_net_rotation: Quat = Quat::from_rotation_y(msg.new.yaw);
        let now = time.elapsed_secs_f64();

        // Remote: update authoritative net transform each snapshot, but DO NOT snap the predicted pose here.
        //
        // The predicted pose for remotes should be advanced smoothly by the extrapolation system
        // between server ticks. Snapping it here forces visible 20Hz stepping.
        //
        // Scope the ParamSet borrows explicitly to avoid holding a borrow across the whole loop iteration.
        {
            let mut remote_q = q.p1();
            if let Ok((mut net_transform, mut predicted, mut last_recv)) =
                remote_q.get_mut(bevy_entity)
            {
                net_transform.translation = new_net_translation;
                net_transform.rotation = new_net_rotation;

                // Keep `PredictedTransform` continuous; extrapolation will converge toward net.
                // If you want a safety snap for large errors, do it in the extrapolation/presentation layer.
                let _ = &mut predicted;

                last_recv.secs = now;
                continue;
            }
        }

        // Local: DO NOT overwrite the predicted pose here.
        // Only update the authoritative net state + accumulate a decaying visual correction offset (Option B).
        //
        // Important: keep the ParamSet borrow (`local_q`) alive for the whole time we use the
        // returned `Mut<>` references. Do not move them out into locals.
        {
            let mut local_q = q.p0();
            let Ok((mut net_transform, predicted, mut offset, mut last_recv)) =
                local_q.get_mut(bevy_entity)
            else {
                continue;
            };

            // Always store the latest authoritative snapshot.
            net_transform.translation = new_net_translation;
            net_transform.rotation = new_net_rotation;
            last_recv.secs = now;

            // Compute prediction error at the moment of receiving authoritative state.
            // Accumulate into the visual offset which will decay smoothly to zero.
            let translation_error = new_net_translation - predicted.translation;

            // Yaw error: compare current predicted yaw with server yaw and accumulate the shortest-angle delta.
            let (pred_yaw, _, _) = predicted.rotation.to_euler(EulerRot::YXZ);
            let (net_yaw, _, _) = new_net_rotation.to_euler(EulerRot::YXZ);
            let mut yaw_error = net_yaw - pred_yaw;

            // Wrap to [-pi, pi] via atan2(sin, cos) of the angle delta.
            yaw_error = yaw_error.sin().atan2(yaw_error.cos());

            offset.translation += translation_error;
            offset.yaw += yaw_error;
        }
    }
}

fn apply_predicted_to_render_transform(
    time: Res<Time>,
    mut q: Query<(
        &mut Transform,
        &PredictedTransform,
        &mut PredictionErrorOffset,
    )>,
) {
    let dt = time.delta_secs();

    // How quickly the correction offset decays toward zero (higher = snappier).
    // With 60Hz, ~14 feels responsive but still smooth.
    let k_translation = 14.0_f32;
    let k_yaw = 18.0_f32;

    // Exponential decay factor: offset *= exp(-k * dt)
    let decay_translation = (-k_translation * dt).exp();
    let decay_yaw = (-k_yaw * dt).exp();

    for (mut transform, predicted, mut offset) in &mut q {
        // Decay the visual error toward zero.
        offset.translation *= decay_translation;
        offset.yaw *= decay_yaw;

        // Present pose = predicted pose + decaying correction.
        transform.translation = predicted.translation + offset.translation;
        transform.rotation = predicted.rotation * Quat::from_rotation_y(offset.yaw);
    }
}
