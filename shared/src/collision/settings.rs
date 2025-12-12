/*!
Kinematic character controller (KCC) settings and tolerances.

These constants centralize the parameters used by the kinematic controller,
collision sweep-and-slide, and ground snapping. Keeping them together makes
tuning easier and helps ensure deterministic behavior across platforms.

Notes
- Distances are in meters, time in seconds.
- Favor practical world-space tolerances over machine epsilon for robust behavior.
- If you want per-actor customization, keep these as sensible defaults and
  override from your game data.
*/

/// Separation from surfaces kept when landing or sliding (meters).
/// Too large creates visible gaps; too small risks jitter on contact.
pub const DEFAULT_SKIN: f32 = 0.02;

/// Maximum number of slide iterations per kinematic step.
/// Higher values help with tight corners at the cost of more queries.
pub const DEFAULT_MAX_ITERATIONS: u32 = 4;

/// Minimum squared movement threshold to consider a step meaningful (m^2).
/// Movements below this are treated as zero to avoid tiny oscillations.
pub const MIN_MOVE_SQ: f32 = 1.0e-8;

/// Practical small distance for comparisons (meters).
/// Use for dot-product guards, equality checks in world space, etc.
pub const DIST_EPS: f32 = 1.0e-6;

/// Additional acceptance buffer added to a capsule's radius (meters).
/// This keeps the controller from jittering when extremely close to a target.
pub const ACCEPTANCE_BUFFER: f32 = 0.05;

/// Max downward snap distance to search for ground (meters).
/// Small values keep the controller from snapping through gaps.
pub const SNAP_MAX_DISTANCE: f32 = 0.30;

/// Hover height above detected ground along the ground normal (meters).
/// Prevents exact contact, which reduces jitter and depenetration needs.
pub const SNAP_HOVER_HEIGHT: f32 = 0.02;

/// Default walking speed in meters per second for KCCs that don't override it.
pub const DEFAULT_MOVEMENT_SPEED: f32 = 5.0;

/// Gravity magnitude in meters per second squared (positive value).
/// Integrate as a downward acceleration if you use continuous gravity.
pub const GRAVITY_MPS2: f32 = 9.81;

/// Helper: compute an acceptance radius for a capsule-based controller.
///
/// The acceptance radius is the distance from the capsule center at which
/// the controller considers it has "reached" a target position. This should
/// be at least the capsule radius plus a small buffer to avoid jitter.
#[inline]
pub const fn acceptance_from_capsule(capsule_radius: f32) -> f32 {
    // Use a const fn formula to keep this usable in const contexts.
    // Clamp to non-negative in case inputs are misconfigured.
    if capsule_radius + ACCEPTANCE_BUFFER < 0.0 {
        0.0
    } else {
        capsule_radius + ACCEPTANCE_BUFFER
    }
}
