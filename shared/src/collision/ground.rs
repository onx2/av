use nalgebra as na;
use parry3d::shape as pshape;

use super::{
    broad, narrow_phase,
    types::{CapsuleSpec, Iso, StaticShape, Vec3},
};

/// Keep a capsule hovering above the nearest ground within `max_snap_distance`.
///
/// Parameters:
/// - `statics`: static world shapes to test against.
/// - `capsule`: Y-aligned capsule specification for the actor (radius + half-height).
/// - `pos`: current capsule center position (world-space).
/// - `max_snap_distance`: maximum downward sweep distance (meters).
/// - `hover_height`: distance to offset the capsule along the ground normal after impact (meters).
///
/// Returns:
/// - `(new_position, hit)` where `hit` is true if ground was detected within the sweep.
///
/// Notes:
/// - If no ground is found, returns `(pos, false)`.
/// - `hover_height` should be small (e.g., 0.02) to reduce jitter and avoid exact contact.
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
/// before running the narrow-phase downward sweep.
///
/// Parameters and return value mirror [`snap_capsule_to_ground`].
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

    // Build downward swept AABB.
    let desired = na::Vector3::new(0.0, -max_snap_distance, 0.0);
    let swept = broad::swept_capsule_aabb(capsule.half_height, capsule.radius, pos, desired, 0.0);

    // Subset of statics: planes + finite shapes overlapping the swept AABB.
    let mut subset: Vec<StaticShape> = Vec::new();
    for &idx in &accel.plane_indices {
        subset.push(statics[idx]);
    }
    for idx in broad::query_candidates(accel, &swept) {
        subset.push(statics[idx]);
    }

    snap_capsule_to_ground(&subset, capsule, pos, max_snap_distance, hover_height)
}
