use nalgebra as na;

use crate::collision::{
    CapsuleSpec, MoveRequest, Quat, StaticShape, Vec3, WorldAccel, move_capsule,
    settings::{
        DIST_EPS, FALL_SPEED_MPS, GROUND_PROBE_DISTANCE, MAX_SLOPE_COS, SNAP_HOVER_HEIGHT,
        SNAP_MAX_DISTANCE, STEP_OFFSET,
    },
    snap_to_ground,
};

/// Output of a single `step_movement()` tick.
#[derive(Clone, Copy, Debug)]
pub struct StepMovementResult {
    /// New capsule center position (world space).
    pub new_translation: Vec3,
    /// New yaw-only rotation that faces the direction of planar motion, if motion occurred.
    ///
    /// If `None`, callers should keep the current rotation as-is.
    pub new_rotation: Option<Quat>,
    /// Whether the controller has walkable ground support this tick.
    pub is_grounded: bool,
}

/// Perform one deterministic KCC tick toward `target` (planar intent + constant fall).
///
/// Behavior
/// - Computes planar intent toward the target (XZ) with acceptance clamping.
/// - Applies KCC movement with sweep-and-slide.
/// - Attempts step-up for small ledges (`STEP_OFFSET`) when blocked in XZ.
/// - If not grounded, applies a constant downward velocity (`FALL_SPEED_MPS`) with collision.
/// - Probes for ground and snaps to maintain a small hover height.
/// - Produces a yaw-only facing quaternion from planar movement.
#[inline]
pub fn step_movement(
    statics: &[StaticShape],
    accel: &WorldAccel,
    capsule: CapsuleSpec,
    current: Vec3,
    target: Vec3,
    speed_mps: f32,
    dt_seconds: f32,
    acceptance_radius: f32,
) -> StepMovementResult {
    let dt = dt_seconds.max(0.0);
    let speed = speed_mps.max(0.0);
    let acceptance = acceptance_radius.max(0.0);

    // 0) Determine grounded at the start (derived, not trusted input).
    let hit = probe_ground(statics, accel, capsule, current);
    let is_grounded = hit.is_some();
    // 1) Compute desired planar (XZ) translation toward target with acceptance clamping.
    let desired_xz = compute_desired_planar_translation(current, target, speed, dt, acceptance);

    // 2) Attempt horizontal movement with sweep-and-slide. If blocked, attempt a step-up.
    let (after_xz, used_step) =
        move_with_optional_step(statics, accel, capsule, current, desired_xz);

    // 2.1) Strict slope enforcement:
    // If we're attempting to move up a steep surface, treat it as a wall (no upward gain from XZ movement).
    // NOTE: This is conservative and keeps the controller simple/deterministic.
    let mut after_xz_strict = after_xz;
    if after_xz_strict.y > current.y + DIST_EPS {
        let walkable = hit
            .as_ref()
            .map(|h| h.normal.y >= MAX_SLOPE_COS)
            .unwrap_or(false);

        // Only allow upward movement if we explicitly used step offset OR we have walkable ground support.
        if !used_step && !walkable {
            after_xz_strict.y = current.y;
        }
    }

    // 3) Apply constant fall (vertical-only) when not grounded.
    // FALL_SPEED_MPS is a positive magnitude; falling means moving in -Y.
    let mut after_vertical = after_xz_strict;
    if dt > 0.0 && !is_grounded {
        let dy = -FALL_SPEED_MPS.max(0.0) * dt;
        if dy.abs() > DIST_EPS {
            let vreq = MoveRequest::with_defaults(after_vertical, Vec3::new(0.0, dy, 0.0), capsule);
            after_vertical = move_capsule(statics, accel, vreq).end_pos;
        }
    }

    // 4) Ground snap/probe to keep hover and derive grounded.
    // Use a short probe distance for stable contact; allow a bit more when airborne to latch.
    let max_down = if used_step {
        (GROUND_PROBE_DISTANCE + SNAP_HOVER_HEIGHT).max(0.0)
    } else if is_grounded {
        (GROUND_PROBE_DISTANCE + SNAP_HOVER_HEIGHT).max(0.0)
    } else {
        SNAP_MAX_DISTANCE.max(0.0)
    };

    let (snapped_pos, snap_hit) = snap_to_ground(
        statics,
        accel,
        capsule,
        after_vertical,
        max_down,
        SNAP_HOVER_HEIGHT,
    );

    // Walkability (slope) comes from the hit normal.
    let is_walkable = snap_hit
        .as_ref()
        .map(|h| h.normal.y >= MAX_SLOPE_COS)
        .unwrap_or(false);

    let is_grounded = snap_hit.is_some() && is_walkable;

    // 5) Yaw-only rotation from *intended* planar motion (toward target).
    // This prevents the character from instantly rotating to be perfectly parallel to walls when sliding.
    // Note: We keep the intent planar by clamping target.y to current.y.
    let intended_xz = Vec3::new(target.x - current.x, 0.0, target.z - current.z);
    let new_rotation = yaw_from_planar_delta(intended_xz);

    StepMovementResult {
        new_translation: snapped_pos,
        new_rotation,
        is_grounded,
    }
}

/// Compute the desired planar translation (XZ) toward `target`, clamped by `speed*dt`
/// and stopping at the acceptance radius boundary.
#[inline]
fn compute_desired_planar_translation(
    current: Vec3,
    target: Vec3,
    speed_mps: f32,
    dt_seconds: f32,
    acceptance_radius: f32,
) -> Vec3 {
    // Planar delta only (XZ).
    let delta = Vec3::new(target.x - current.x, 0.0, target.z - current.z);
    let dist = (delta.x * delta.x + delta.z * delta.z).sqrt();

    if dist <= acceptance_radius + DIST_EPS {
        return Vec3::zeros();
    }

    let max_step = speed_mps * dt_seconds;
    if max_step <= DIST_EPS {
        return Vec3::zeros();
    }

    let to_boundary = (dist - acceptance_radius).max(0.0);
    let step = to_boundary.min(max_step);

    // Normalize delta in XZ.
    let dir = Vec3::new(delta.x / dist, 0.0, delta.z / dist);
    dir * step
}

/// Move horizontally with sweep-and-slide; if significantly blocked and step offset is enabled,
/// attempt a step-up:
/// - Up by STEP_OFFSET
/// - Forward by desired_xz
/// - Down by STEP_OFFSET + small snap/probe
///
/// Returns (new_pos, used_step).
#[inline]
fn move_with_optional_step(
    statics: &[StaticShape],
    accel: &WorldAccel,
    capsule: CapsuleSpec,
    start: Vec3,
    desired_xz: Vec3,
) -> (Vec3, bool) {
    if desired_xz.norm_squared() <= DIST_EPS * DIST_EPS {
        return (start, false);
    }

    // First try moving purely in XZ.
    let req = MoveRequest::with_defaults(start, desired_xz, capsule);
    let moved = move_capsule(statics, accel, req);
    let after = moved.end_pos;

    // If we got most of the desired move, don't bother stepping.
    let achieved = after - start;
    let achieved_xz = Vec3::new(achieved.x, 0.0, achieved.z);
    let desired_len = desired_xz.norm();
    let achieved_len = achieved_xz.norm();

    if desired_len <= DIST_EPS || achieved_len >= desired_len * 0.9 || STEP_OFFSET <= 0.0 {
        return (after, false);
    }

    // Step attempt: up -> forward -> down.
    // Up
    let up_req = MoveRequest::with_defaults(start, Vec3::new(0.0, STEP_OFFSET, 0.0), capsule);
    let up = move_capsule(statics, accel, up_req).end_pos;

    // Forward from the stepped-up position
    let fwd_req = MoveRequest::with_defaults(up, desired_xz, capsule);
    let fwd = move_capsule(statics, accel, fwd_req).end_pos;

    // Down (try to settle back)
    let down_req = MoveRequest::with_defaults(
        fwd,
        Vec3::new(0.0, -(STEP_OFFSET + GROUND_PROBE_DISTANCE).max(0.0), 0.0),
        capsule,
    );
    let down = move_capsule(statics, accel, down_req).end_pos;

    // If stepping produced a better horizontal result, accept it.
    let step_achieved = down - start;
    let step_achieved_xz = Vec3::new(step_achieved.x, 0.0, step_achieved.z);
    if step_achieved_xz.norm() > achieved_len + 1.0e-4 {
        return (down, true);
    }

    (after, false)
}

/// Probe for ground support under `pos`.
///
/// Returns `(is_hit, hit)` where `hit` is the best ground contact (including the surface normal)
/// within the probe distance. If no ground is found, `hit` is `None`.
#[inline]
fn probe_ground(
    statics: &[StaticShape],
    accel: &WorldAccel,
    capsule: CapsuleSpec,
    pos: Vec3,
) -> Option<crate::collision::MoveHit> {
    let (_snapped, hit) = snap_to_ground(
        statics,
        accel,
        capsule,
        pos,
        (GROUND_PROBE_DISTANCE + SNAP_HOVER_HEIGHT).max(0.0),
        SNAP_HOVER_HEIGHT,
    );

    hit
}

/// Compute a yaw-only rotation (about +Y) that faces `delta_xz`.
///
/// Returns `None` if the planar delta is too small.
#[inline]
fn yaw_from_planar_delta(delta_xz: Vec3) -> Option<Quat> {
    let len_sq = delta_xz.norm_squared();
    if len_sq <= DIST_EPS * DIST_EPS {
        return None;
    }

    // Match existing server convention (see older tick reducer logic):
    // server used: yaw = (-dx).atan2(-dz)
    // where dx,dz are the intended planar displacement components.
    //
    // This flips the facing direction compared to the naive atan2(dx, dz) and fixes the
    // "opposite direction" issue you're seeing.
    let yaw = (-delta_xz.x).atan2(-delta_xz.z);

    Some(na::UnitQuaternion::from_axis_angle(
        &na::Vector3::y_axis(),
        yaw,
    ))
}
