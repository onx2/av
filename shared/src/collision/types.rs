/*!
Core collision types and math aliases shared by the collision submodules.

This module intentionally contains no algorithms. It defines the data types
exchanged between:
- broad_phase (e.g., static world acceleration structures and candidate queries)
- narrow_phase (e.g., parry3d time-of-impact and intersection queries)
- kinematic controller (sweep-and-slide)
- ground snapping
- higher-level motion helpers

Notes on future extensibility:
- Triggers and “zones” (e.g., swamps, invulnerability regions) are best modeled
  as sensor shapes processed during the tick. A typical pattern is:
  - Broad-phase prune to candidate sensors.
  - Narrow-phase intersection_test to confirm overlap.
  - Emit deterministic “enter/exit” events or apply effects (e.g., speed scaling).
- Contact graphs aggregate persistent pairs across frames for dynamic worlds.
  In a purely kinematic, server-authoritative setup, you can often compute
  overlaps/sweeps deterministically each tick without maintaining a persistent
  graph. If you later introduce many dynamic bodies or complex contact lifetimes,
  a contact graph becomes valuable.
*/

use nalgebra as na;

/// Common math aliases for clarity and consistency.
pub type Vec3 = na::Vector3<f32>;
pub type Quat = na::UnitQuaternion<f32>;
pub type Iso = na::Isometry3<f32>;

/// A rigid transform (isometry) in world space.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Transform {
    #[inline]
    pub fn new(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
        }
    }

    /// Convert to nalgebra `Isometry3` for use with parry3d narrow-phase queries.
    #[inline]
    pub fn iso(&self) -> Iso {
        Iso::from_parts(
            na::Translation3::new(self.translation.x, self.translation.y, self.translation.z),
            self.rotation,
        )
    }
}

/// Static collision shapes supported by the world.
///
/// - Plane: infinite plane in world-space represented by its normal and offset (dist)
///          satisfying: normal ⋅ x = dist.
/// - Cuboid: oriented box with half-extents in local space, placed by `transform`.
#[derive(Clone, Copy, Debug)]
pub enum StaticShape {
    Plane {
        /// World-space unit normal of the plane.
        normal: Vec3,
        /// Plane offset along the normal, i.e., normal ⋅ x = dist.
        dist: f32,
    },
    Cuboid {
        /// Local-space half-extents (hx, hy, hz).
        half_extents: Vec3,
        /// World-space pose of the cuboid.
        transform: Transform,
    },
    Sphere {
        /// Radius of the sphere in meters.
        radius: f32,
        /// World-space pose (translation used; rotation ignored).
        transform: Transform,
    },
    Capsule {
        /// Radius of the spherical caps and cylinder.
        radius: f32,
        /// Half of the cylinder length along the local +Y axis.
        half_height: f32,
        /// World-space pose of the capsule.
        transform: Transform,
    },
}

/// Capsule specification for kinematic actors.
///
/// half_height is the half-length of the cylinder section (aligned with +Y),
/// so the total capsule height is 2*half_height + 2*radius.
#[derive(Clone, Copy, Debug)]
pub struct CapsuleSpec {
    pub radius: f32,
    pub half_height: f32,
}

/// A single contact result returned by a time-of-impact (TOI) query
/// used during sweep-and-slide or ground snapping.
#[derive(Clone, Copy, Debug)]
pub struct MoveHit {
    /// World-space contact normal on the moving shape.
    pub normal: Vec3,
    /// Fraction (0..1) of the tested translation where the hit occurred.
    pub fraction: f32,
}

/// Result of a kinematic movement step (after sweep-and-slide).
#[derive(Clone, Copy, Debug)]
pub struct MoveResult {
    /// Final capsule center position after applying the step and sliding.
    pub end_pos: Vec3,
    /// Information about the last hit encountered during the step (if any).
    pub last_hit: Option<MoveHit>,
    /// Remaining translation that could not be consumed (usually zero on success).
    pub remaining: Vec3,
}
