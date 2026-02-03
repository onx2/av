use super::Vec3;
use rapier3d::prelude::{SharedShape, Vector};
use spacetimedb::SpacetimeType;

/// Y-aligned capsule collider definition
///
/// Semantics:
/// - `radius`: radius of spherical caps and cylinder.
/// - `half_height`: half of the cylinder length along local +Y.
/// - Total capsule height = `2*half_height + 2*radius`.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct CapsuleY {
    pub radius: f32,
    pub half_height: f32,
}

/// Cylinder dimensions for collider definitions (Y-aligned).
///
/// Semantics:
/// - `radius`: radius of the cylinder.
/// - `half_height`: half of the cylinder length along local +Y.
/// - Total height = `2*half_height`.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct Cylinder {
    pub radius: f32,
    pub half_height: f32,
}

/// Cone dimensions for collider definitions (Y-aligned).
///
/// Semantics:
/// - `radius`: radius of the cone base.
/// - `half_height`: half of the cone height along local +Y.
/// - Total height = `2*half_height`.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    pub radius: f32,
    pub half_height: f32,
}

/// Rounded-box parameters.
///
/// Semantics:
/// - `half_extents`: half extents of the cuboid (hx, hy, hz).
/// - `border_radius`: rounding radius applied to edges/corners.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct RoundCuboid {
    pub half_extents: Vec3,
    pub border_radius: f32,
}

/// Rounded cylinder parameters (Y-aligned).
///
/// Semantics:
/// - `radius`: radius of the cylinder.
/// - `half_height`: half of the cylinder height along +Y.
/// - `border_radius`: rounding radius applied to edges/caps.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct RoundCylinder {
    pub radius: f32,
    pub half_height: f32,
    pub border_radius: f32,
}

/// Rounded cone parameters (Y-aligned).
///
/// Semantics:
/// - `radius`: base radius.
/// - `half_height`: half of the cone height along +Y.
/// - `border_radius`: rounding radius applied to edges.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct RoundCone {
    pub radius: f32,
    pub half_height: f32,
    pub border_radius: f32,
}

/// Collider shape used by world statics (and potentially triggers in the future).
///
/// Notes:
/// - Variants are newtype-like to keep storage compact and easy to serialize.
/// - Shapes are combined with per-row `translation`, `rotation`, and `scale`.
/// - For "plane size" in Bevy: Rapier planes/half-spaces are infinite; any X/Z size is visual only.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub enum ColliderShape {
    /// Infinite plane (half-space). `f32` is the offset along the plane normal:
    /// the plane satisfies `n ⋅ x = dist`, where `n = rotation * +Y`.
    Plane(f32),
    /// Oriented box defined by local half-extents (hx, hy, hz).
    /// The final physics size used by the server is `half_extents * scale`.
    Cuboid(Vec3),
    /// Sphere/ball with the given radius (meters).
    Sphere(f32),
    /// Y-aligned capsule with `radius` and `half_height`.
    CapsuleY(CapsuleY),
    /// Y-aligned cylinder.
    Cylinder(Cylinder),
    /// Y-aligned cone.
    Cone(Cone),
    /// Rounded cuboid (box with rounded edges/corners).
    RoundCuboid(RoundCuboid),
    /// Rounded Y-aligned cylinder.
    RoundCylinder(RoundCylinder),
    /// Rounded Y-aligned cone.
    RoundCone(RoundCone),
}

impl From<ColliderShape> for SharedShape {
    fn from(shape: ColliderShape) -> Self {
        match shape {
            // So we build a +Y halfspace here and rely on the caller to apply translation/rotation.
            ColliderShape::Plane(_offset) => SharedShape::halfspace(Vector::y_axis()),
            ColliderShape::Cuboid(half_extents) => {
                SharedShape::cuboid(half_extents.x, half_extents.y, half_extents.z)
            }
            ColliderShape::Sphere(radius) => SharedShape::ball(radius),
            ColliderShape::CapsuleY(c) => SharedShape::capsule_y(c.half_height, c.radius),
            ColliderShape::Cylinder(c) => SharedShape::cylinder(c.half_height, c.radius),
            ColliderShape::Cone(c) => SharedShape::cone(c.half_height, c.radius),
            ColliderShape::RoundCuboid(c) => SharedShape::round_cuboid(
                c.half_extents.x,
                c.half_extents.y,
                c.half_extents.z,
                c.border_radius,
            ),
            ColliderShape::RoundCylinder(c) => {
                SharedShape::round_cylinder(c.half_height, c.radius, c.border_radius)
            }
            ColliderShape::RoundCone(c) => {
                SharedShape::round_cone(c.half_height, c.radius, c.border_radius)
            }
        }
    }
}
