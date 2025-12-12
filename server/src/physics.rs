#![allow(dead_code)]
/// Physics helpers for the server tick: thin, well-documented wrappers around the
/// shared kinematic character controller (KCC) and ground-snap logic.
///
/// Design
/// - These functions do not own state; they take the inputs required for a single
///   controller step and delegate to the shared KCC (in the `shared` crate).
/// - World statics and their broad-phase accelerator are retrieved via `crate::world`,
///   which caches immutable data with a lock-free, one-time initialization.
/// - The goal is to keep the tick reducer readable by expressing the common
///   “move → collide → snap → gravity” sequence through small, composable calls.
use nalgebra as na;
use shared::collision::{
    self,
    kinematic::MoveRequest,
    settings::{DEFAULT_MAX_ITERATIONS, DEFAULT_SKIN, SNAP_HOVER_HEIGHT, SNAP_MAX_DISTANCE},
    CapsuleSpec, MoveResult,
};
use shared::motion;

use spacetimedb::ReducerContext;

use crate::world::{world_accel, world_statics};

/// Compute the horizontal (planar) desired translation toward `target`, clamped by
/// `speed_mps * dt_seconds` and an acceptance radius derived from `capsule_radius`.
///
/// This is an intent-layer helper (motion), not a collision operation. Typically you:
/// 1) Call this to obtain a desired planar step (Y = 0).
/// 2) Feed the result into a kinematic sweep-and-slide step (collision).
///
/// Returns a vector with Y = 0.
///
/// Rationale
/// - The acceptance radius is set from the capsule radius plus a small buffer to avoid
///   oscillations/jitter when near the target.
/// - The Y component is zeroed so this does not request vertical motion. Gravity and
///   ground snapping are handled separately.
///
/// Example
/// ```ignore
/// let desired = compute_desired_horizontal(current, target, 5.0, dt, capsule.radius);
/// let step   = kcc_horizontal_step_with_defaults(ctx, start_pos, desired, capsule);
/// ```
pub fn compute_desired_horizontal(
    current: na::Point3<f32>,
    target: na::Point3<f32>,
    speed_mps: f32,
    dt_seconds: f32,
    capsule_radius: f32,
) -> na::Vector3<f32> {
    let plan = motion::compute_desired_with_capsule_acceptance(
        current,
        // Keep navigation planar by clamping target.y to current.y.
        na::Point3::new(target.x, current.y, target.z),
        speed_mps,
        dt_seconds,
        capsule_radius,
    );
    // Zero vertical intent; vertical control is gravity/snap.
    na::Vector3::new(plan.desired_translation.x, 0.0, plan.desired_translation.z)
}

/// Perform a single kinematic sweep-and-slide step with default `skin` and
/// `max_iterations`, pruning candidate statics via the cached world accelerator.
///
/// Inputs
/// - `start_pos`: capsule center at the start of the step.
/// - `desired_translation`: desired displacement (meters) for this tick, typically planar.
/// - `capsule`: capsule dimensions used for collision.
///
/// Returns `MoveResult` with the new capsule center, last hit info, and any
/// remaining (unconsumed) translation.
///
/// See also: [`kcc_horizontal_step`] for a variant with explicit skin/iterations.
pub fn kcc_horizontal_step_with_defaults(
    ctx: &ReducerContext,
    start_pos: na::Vector3<f32>,
    desired_translation: na::Vector3<f32>,
    capsule: CapsuleSpec,
) -> MoveResult {
    kcc_horizontal_step(
        ctx,
        start_pos,
        desired_translation,
        capsule,
        DEFAULT_SKIN,
        DEFAULT_MAX_ITERATIONS,
    )
}

/// Perform a single kinematic sweep-and-slide step with explicit `skin` and `max_iterations`,
/// pruning candidate statics via the cached world accelerator.
///
/// Notes
/// - Uses the shared KCC (`move_capsule_kinematic_with_accel`) which performs a parry3d
///   time-of-impact cast, lands at contact minus skin, and slides along the contact normal.
/// - Iterates up to `max_iterations` to handle corners robustly.
pub fn kcc_horizontal_step(
    ctx: &ReducerContext,
    start_pos: na::Vector3<f32>,
    desired_translation: na::Vector3<f32>,
    capsule: CapsuleSpec,
    skin: f32,
    max_iterations: u32,
) -> MoveResult {
    let statics = world_statics(ctx);
    let accel = world_accel(ctx);

    let req = MoveRequest {
        start_pos,
        desired_translation,
        capsule,
        skin,
        max_iterations,
    };
    collision::move_capsule_kinematic_with_accel(statics, accel, req)
}

/// Cast the capsule down and keep it hovering slightly above the surface along the contact normal,
/// e.g., to land after moving or to clamp tiny residual vertical gaps.
///
/// Inputs
/// - `pos`: current capsule center (meters).
/// - `capsule`: controller capsule dimensions.
/// - `max_snap_distance`: maximum downward cast distance (meters).
/// - `hover_height`: offset along the surface normal after impact (meters).
///
/// Returns `(new_pos, hit)` where `hit = true` indicates ground was detected.
///
/// Typical usage
/// - After horizontal movement: snap using a small distance (e.g., 0.3m) and hover (e.g., 0.02m).
pub fn snap_to_ground(
    ctx: &ReducerContext,
    pos: na::Vector3<f32>,
    capsule: CapsuleSpec,
    max_snap_distance: f32,
    hover_height: f32,
) -> (na::Vector3<f32>, bool) {
    let statics = world_statics(ctx);
    let accel = world_accel(ctx);
    collision::snap_capsule_to_ground_with_accel(
        statics,
        accel,
        capsule,
        pos,
        max_snap_distance,
        hover_height,
    )
}

/// Apply a gravity-only step when airborne:
/// - Move down by `fall_speed_mps * dt_seconds` (negative speed for downward direction).
/// - Resolve collision via the KCC (vertical-only).
/// - Try a final ground snap to latch if within range.
///
/// Returns `(final_pos, grounded)` where `grounded` indicates whether a snap found ground.
///
/// Rationale
/// - While falling, horizontal motion is typically suppressed or canceled. This function
///   does not take a horizontal displacement by design.
/// - You can tune `skin`, `max_iterations`, `snap_max_distance`, and `snap_hover_height`
///   as needed. For a quick default, use `DEFAULT_SKIN`, `DEFAULT_MAX_ITERATIONS`,
///   `SNAP_MAX_DISTANCE`, and `SNAP_HOVER_HEIGHT`.
pub fn gravity_step(
    ctx: &ReducerContext,
    start_pos: na::Vector3<f32>,
    capsule: CapsuleSpec,
    fall_speed_mps: f32,
    dt_seconds: f32,
    skin: f32,
    max_iterations: u32,
    snap_max_distance: f32,
    snap_hover_height: f32,
) -> (na::Vector3<f32>, bool) {
    let statics = world_statics(ctx);
    let accel = world_accel(ctx);

    let fall_desired = na::Vector3::new(0.0, fall_speed_mps * dt_seconds, 0.0);
    let fall_req = MoveRequest {
        start_pos,
        desired_translation: fall_desired,
        capsule,
        skin,
        max_iterations,
    };

    let fall_col = collision::move_capsule_kinematic_with_accel(statics, accel, fall_req);
    let (snapped_pos, hit) = collision::snap_capsule_to_ground_with_accel(
        statics,
        accel,
        capsule,
        fall_col.end_pos,
        snap_max_distance,
        snap_hover_height,
    );
    (if hit { snapped_pos } else { fall_col.end_pos }, hit)
}

/// Convenience wrapper around [`gravity_step`] using default settings for skin,
/// iterations, and snap distances.
///
/// Returns `(final_pos, grounded)`.
pub fn gravity_step_with_defaults(
    ctx: &ReducerContext,
    start_pos: na::Vector3<f32>,
    capsule: CapsuleSpec,
    fall_speed_mps: f32,
    dt_seconds: f32,
) -> (na::Vector3<f32>, bool) {
    gravity_step(
        ctx,
        start_pos,
        capsule,
        fall_speed_mps,
        dt_seconds,
        DEFAULT_SKIN,
        DEFAULT_MAX_ITERATIONS,
        SNAP_MAX_DISTANCE,
        SNAP_HOVER_HEIGHT,
    )
}
