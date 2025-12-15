use nalgebra as na;
use parry3d::shape as pshape;

use super::{
    broad, narrow_phase,
    types::{CapsuleSpec, Iso, MoveHit, StaticShape, Vec3},
};

/// Snap the capsule down onto the nearest ground within `max_snap_distance`,
/// then keep it hovering above the surface by `hover_height` along the contact normal.
///
/// This is the only supported ground-snapping path in `shared`; it always uses the
/// prebuilt broad-phase accelerator to stay fast.
///
/// - `pos` is the current capsule center (world space).
/// - If a hit is found within the downward cast range, the capsule center is moved to the
///   impact position, offset by `hover_height` along the surface normal.
/// - If no hit is found, returns `pos` unchanged.
///
/// Returns `(new_pos, hit)` where `hit` contains the contact normal and fraction (if any).
pub fn snap_to_ground(
    statics: &[StaticShape],
    accel: &broad::WorldAccel,
    capsule: CapsuleSpec,
    pos: Vec3,
    max_snap_distance: f32,
    hover_height: f32,
) -> (Vec3, Option<MoveHit>) {
    if max_snap_distance <= 0.0 || hover_height < 0.0 {
        return (pos, None);
    }

    // Downward sweep parameters.
    let desired = na::Vector3::new(0.0, -max_snap_distance, 0.0);
    let swept = broad::swept_capsule_aabb(capsule.half_height, capsule.radius, pos, desired, 0.0);

    let capsule_shape = pshape::Capsule::new_y(capsule.half_height, capsule.radius);
    let capsule_iso: Iso = Iso::from_parts(
        na::Translation3::new(pos.x, pos.y, pos.z),
        na::UnitQuaternion::identity(),
    );
    let vel = desired;

    // Find earliest downward hit from planes and finite candidates.
    let mut best: Option<MoveHit> = None;

    // Test planes (infinite; not in accel).
    for &idx in &accel.plane_indices {
        if let Some(hit) = narrow_phase::cast_capsule_against_static(
            capsule_iso,
            &capsule_shape,
            vel,
            1.0,
            &statics[idx],
        ) {
            if best.as_ref().map_or(true, |b| hit.fraction < b.fraction) {
                best = Some(hit);
            }
        }
    }

    // Test finite shapes from broad-phase candidate indices.
    let candidates = broad::query_candidates(accel, &swept);
    for idx in candidates {
        if let Some(hit) = narrow_phase::cast_capsule_against_static(
            capsule_iso,
            &capsule_shape,
            vel,
            1.0,
            &statics[idx],
        ) {
            if best.as_ref().map_or(true, |b| hit.fraction < b.fraction) {
                best = Some(hit);
            }
        }
    }

    if let Some(mut hit) = best {
        // Impact center (fraction along cast).
        let impact_center = pos + vel * hit.fraction;

        // Ensure the normal opposes motion (consistent with slide logic).
        if hit.normal.dot(&vel) > 0.0 {
            hit.normal = -hit.normal;
        }

        // Hover slightly above ground along the contact normal.
        let new_pos = impact_center + hit.normal * hover_height.max(0.0);
        return (new_pos, Some(hit));
    }

    (pos, None)
}
