/// Capsule dimensions for collider definitions.
///
/// Semantics:
/// - `radius`: radius of spherical caps and cylinder.
/// - `half_height`: half of the cylinder length along local +Y.
/// - Total capsule height = `2*half_height + 2*radius`.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbCapsule {
    pub radius: f32,
    pub half_height: f32,
}

/// Cylinder dimensions for collider definitions (Y-aligned).
///
/// Semantics:
/// - `radius`: radius of the cylinder.
/// - `half_height`: half of the cylinder length along local +Y.
/// - Total height = `2*half_height`.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbCylinder {
    pub radius: f32,
    pub half_height: f32,
}

/// Cone dimensions for collider definitions (Y-aligned).
///
/// Semantics:
/// - `radius`: radius of the cone base.
/// - `half_height`: half of the cone height along local +Y.
/// - Total height = `2*half_height`.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbCone {
    pub radius: f32,
    pub half_height: f32,
}

/// Rounded-box parameters.
///
/// Semantics:
/// - `half_extents`: half extents of the cuboid (hx, hy, hz).
/// - `border_radius`: rounding radius applied to edges/corners.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbRoundCuboid {
    pub half_extents: super::DbVec3,
    pub border_radius: f32,
}

/// Rounded cylinder parameters (Y-aligned).
///
/// Semantics:
/// - `radius`: radius of the cylinder.
/// - `half_height`: half of the cylinder height along +Y.
/// - `border_radius`: rounding radius applied to edges/caps.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbRoundCylinder {
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
#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbRoundCone {
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
#[derive(spacetimedb::SpacetimeType, PartialEq)]
pub enum ColliderShape {
    /// Infinite plane (half-space). `f32` is the offset along the plane normal:
    /// the plane satisfies `n ⋅ x = dist`, where `n = rotation * +Y`.
    Plane(f32),
    /// Oriented box defined by local half-extents (hx, hy, hz).
    /// The final physics size used by the server is `half_extents * scale`.
    Cuboid(super::DbVec3),
    /// Sphere/ball with the given radius (meters).
    Sphere(f32),
    /// Y-aligned capsule with `radius` and `half_height`.
    Capsule(DbCapsule),
    /// Y-aligned cylinder.
    Cylinder(DbCylinder),
    /// Y-aligned cone.
    Cone(DbCone),
    /// Rounded cuboid (box with rounded edges/corners).
    RoundCuboid(DbRoundCuboid),
    /// Rounded Y-aligned cylinder.
    RoundCylinder(DbRoundCylinder),
    /// Rounded Y-aligned cone.
    RoundCone(DbRoundCone),
}
