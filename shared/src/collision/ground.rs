use nalgebra as na;
use parry3d::shape as pshape;

use super::{
    broad, narrow_phase,
    types::{CapsuleSpec, Iso, StaticShape, Vec3},
};

/// Keep a capsule hovering above the nearest ground within `max_snap_distance`.
/// - `pos` is the current capsule center (world space).
/// - If a hit is found within the downward cast range, the capsule center is moved to the
///   impact position, offset by `hover_height` along the surface normal.
/// - If no hit is found, returns `pos` unchanged.
pub fn snap_capsule_to_ground(
    statics: &[StaticShape],
    capsule: CapsuleSpec,
    pos: Vec3,
    max_snap_distance: f32,
    hover_height: f32,
) -> (Vec3, bool) {
    if max_snap_distance <= 0.0 || hover_height < 0.0 {
        return (pos, false);
    }

    let capsule_shape = pshape::Capsule::new_y(capsule.half_height, capsule.radius);
    let capsule_iso: Iso = Iso::from_parts(
        na::Translation3::new(pos.x, pos.y, pos.z),
        na::UnitQuaternion::identity(),
    );
    let vel = na::Vector3::new(0.0, -max_snap_distance, 0.0);

    // Find earliest downward hit.
    let mut best: Option<super::types::MoveHit> = None;
    for s in statics {
        if let Some(hit) =
            narrow_phase::cast_capsule_against_static(capsule_iso, &capsule_shape, vel, 1.0, s)
        {
            if best.as_ref().map_or(true, |b| hit.fraction < b.fraction) {
                best = Some(hit);
            }
        }
    }

    if let Some(hit) = best {
        // Impact center (fraction along cast).
        let impact_center = pos + vel * hit.fraction;

        // Ensure the normal opposes motion (consistent with slide logic).
        let mut n = hit.normal;
        if n.dot(&vel) > 0.0 {
            n = -n;
        }

        // Hover slightly above ground along the contact normal.
        let new_pos = impact_center * 1.0 + n * hover_height.max(0.0);
        return (new_pos, true);
    }

    (pos, false)
}

/// Broad-phase accelerated ground snapping.
///
/// Uses a prebuilt accelerator to prune candidate statics (planes + overlapping AABBs)
/// before running the narrow-phase downward sweep, without cloning shapes.
/// Iterates candidate indices directly and tests against `statics[idx]`.
pub fn snap_capsule_to_ground_with_accel(
    statics: &[StaticShape],
    accel: &broad::WorldAccel,
    capsule: CapsuleSpec,
    pos: Vec3,
    max_snap_distance: f32,
    hover_height: f32,
) -> (Vec3, bool) {
    if max_snap_distance <= 0.0 || hover_height < 0.0 {
        return (pos, false);
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
    let mut best: Option<super::types::MoveHit> = None;

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

    if let Some(hit) = best {
        // Impact center (fraction along cast).
        let impact_center = pos + vel * hit.fraction;

        // Ensure the normal opposes motion (consistent with slide logic).
        let mut n = hit.normal;
        if n.dot(&vel) > 0.0 {
            n = -n;
        }

        // Hover slightly above ground along the contact normal.
        let new_pos = impact_center + n * hover_height.max(0.0);
        return (new_pos, true);
    }

    (pos, false)
}
