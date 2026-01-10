use super::NetworkTransform;
use bevy::prelude::*;

/// Smoothly interpolate rendered transforms toward the latest network state.
///
/// This keeps visuals stable even if server updates arrive at a lower rate than rendering.
/// We use:
/// - `smooth_nudge` for translation (critically damped-ish smoothing),
/// - `slerp` for rotation, with an exponential smoothing factor.
///
/// Note: `NetworkActor` is the authoritative "latest received" snapshot from the server.
/// This system drives the scene `Transform` toward that snapshot.
pub(super) fn interpolate(
    time: Res<Time>,
    mut transform_q: Query<(&mut Transform, &NetworkTransform)>,
) {
    let dt = time.delta_secs();
    transform_q.par_iter_mut().for_each(|(mut transform, net)| {
        transform
            .translation
            .smooth_nudge(&net.translation, 8.0, dt);
        transform.rotation = transform
            .rotation
            .slerp(net.rotation, 1.0 - (-24.0 * dt).exp());
    });
}
