/*!
Collision root module.

This module re-exports submodules that implement the kinematic character
controller (KCC) using parry3d for narrow-phase queries and a simple broad-phase
for static world acceleration.

Public API policy
-----------------
This crate is intended to expose only a minimal public surface area. Movement
code should use the accelerated variants (with `WorldAccel`) to ensure the
client and server stay fast and deterministic.

The code is split for clarity:

- types:        shared data types (Transform, StaticShape, CapsuleSpec, etc.)
- settings:     controller and tolerance constants
- broad:        broad-phase helpers (swept AABBs, candidate queries)
- narrow_phase: thin wrappers over parry3d queries (TOI, intersections, rays)
- kinematic:    sweep-and-slide controller
- ground:       downward snap and grounded logic
*/

pub mod broad;
pub mod ground;
pub mod kinematic;
pub mod narrow_phase;
pub mod settings;
pub mod types;

// Re-export commonly used types and functions.
//
// Public API: only fast paths are exposed.
pub use broad::{WorldAccel, build_world_accel};
pub use ground::snap_to_ground;
pub use kinematic::{MoveRequest, move_capsule};
pub use types::{CapsuleSpec, MoveHit, MoveResult, Quat, StaticShape, Transform, Vec3};

/// Convenience: build a `StaticShape::Plane` from a world-space plane pose:
/// - normal = rotation * +Y
/// - dist = dot(normal, translation) + optional offset
#[inline]
pub fn plane_from_pose(rotation: Quat, translation: Vec3, offset_along_normal: f32) -> StaticShape {
    let normal = rotation * Vec3::new(0.0, 1.0, 0.0);
    let dist = normal.dot(&translation) + offset_along_normal;
    StaticShape::Plane { normal, dist }
}

/// Convenience: build a `StaticShape::Cuboid` with given half extents and pose.
#[inline]
pub fn cuboid_from_pose(half_extents: Vec3, translation: Vec3, rotation: Quat) -> StaticShape {
    StaticShape::Cuboid {
        half_extents,
        transform: Transform {
            translation,
            rotation,
        },
    }
}

#[inline]
pub fn sphere_from_pose(radius: f32, translation: Vec3, rotation: Quat) -> StaticShape {
    StaticShape::Sphere {
        radius,
        transform: Transform {
            translation,
            rotation,
        },
    }
}

#[inline]
pub fn capsule_from_pose(
    radius: f32,
    half_height: f32,
    translation: Vec3,
    rotation: Quat,
) -> StaticShape {
    StaticShape::Capsule {
        radius,
        half_height,
        transform: Transform {
            translation,
            rotation,
        },
    }
}
