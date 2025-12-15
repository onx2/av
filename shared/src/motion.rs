use nalgebra as na;

// NOTE: This module is an internal implementation detail.
// Public acceptance helpers live in `collision::settings`.
//
// Keep this file focused on "desired translation" math; do not re-export
// collision settings or helper functions from here.
use crate::collision::settings::{DIST_EPS, acceptance_from_capsule};

/// Input for computing the desired translation toward a target in 3D.
///
/// - The movement is clamped so we stop at the surface of the acceptance sphere (never overlapping).
/// - If already within acceptance, we return zero translation and `finished = true`.
#[derive(Clone, Copy, Debug)]
pub struct MoveTowardParams {
    /// Current world position of the mover (meters).
    pub current: na::Point<f32, 3>,
    /// Target world position (meters).
    pub target: na::Point<f32, 3>,
    /// Linear speed in meters per second.
    pub speed_mps: f32,
    /// Delta time in seconds.
    pub dt_seconds: f32,
    /// Acceptance radius (meters). If `distance(current, target) <= acceptance_radius`,
    /// the mover is considered done.
    pub acceptance_radius: f32,
}

/// Result of the "desired translation" computation.
#[derive(Clone, Copy, Debug)]
pub struct MoveTowardResult {
    /// The translation we want to apply for this tick (meters).
    /// This is clamped so we never overshoot the acceptance boundary.
    pub desired_translation: na::Vector3<f32>,
    /// Are we within acceptance after this step decision?
    /// - If already within acceptance at the start, this is true and desired_translation is zero.
    /// - If we can reach the acceptance boundary this tick, this is true.
    /// - Otherwise, this is false (more movement needed next tick).
    pub finished: bool,
    /// Current distance to the target at the time of computation (meters).
    pub distance_to_target: f32,
}

/// Compute the desired translation toward `target` at `speed_mps` over `dt_seconds`,
/// stopping on the acceptance sphere boundary (never overlapping).
///
/// This does not apply any collision. Feed the returned `desired_translation` to your
/// sweep-and-slide function, then commit the final position it computes.
#[inline]
pub fn compute_desired_translation(params: MoveTowardParams) -> MoveTowardResult {
    let MoveTowardParams {
        current,
        target,
        speed_mps,
        dt_seconds,
        acceptance_radius,
    } = params;

    let delta = target - current; // Vector3
    let dist = delta.norm();

    // Clamp inputs.
    let acc = acceptance_radius.max(0.0);
    let speed = speed_mps.max(0.0);
    let dt = dt_seconds.max(0.0);

    // 1) Already within acceptance → no movement, finished.
    if dist <= acc + DIST_EPS {
        return MoveTowardResult {
            desired_translation: na::Vector3::zeros(),
            finished: true,
            distance_to_target: dist,
        };
    }

    // 2) Compute this tick's max movement and how far to the acceptance boundary.
    let max_step = speed * dt;
    if max_step <= DIST_EPS {
        return MoveTowardResult {
            desired_translation: na::Vector3::zeros(),
            finished: false,
            distance_to_target: dist,
        };
    }

    let to_boundary = (dist - acc).max(0.0);
    let step = to_boundary.min(max_step);

    // 3) Direction toward target; safe since dist > acc >= 0.
    let dir = delta / dist;
    let desired = dir * step;

    // If we can reach the boundary this frame, we are finished after this step.
    let finished = (to_boundary - step).abs() <= DIST_EPS;

    MoveTowardResult {
        desired_translation: desired,
        finished,
        distance_to_target: dist,
    }
}

/// Convenience wrapper: compute desired translation with acceptance derived from a capsule radius.
#[inline]
pub fn compute_desired_with_capsule_acceptance(
    current: na::Point3<f32>,
    target: na::Point3<f32>,
    speed_mps: f32,
    dt_seconds: f32,
    capsule_radius: f32,
) -> MoveTowardResult {
    compute_desired_translation(MoveTowardParams {
        current,
        target,
        speed_mps,
        dt_seconds,
        acceptance_radius: acceptance_from_capsule(capsule_radius),
    })
}
