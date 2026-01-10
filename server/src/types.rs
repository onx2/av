use nalgebra as na;
use spacetimedb::{Identity, SpacetimeType};

use crate::schema::Actor;

/// A 3D vector in world space (meters).
///
/// Semantics:
/// - Used for translations, scales, and general scalar triplets.
/// - This is a data type only; math/conversions live outside the schema.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbVec3 {
    /// X axis (east-west)
    pub x: f32,
    /// Y axis (up-down)
    pub y: f32,
    /// Z axis (north-south)
    pub z: f32,
}

impl Default for DbVec3 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl DbVec3 {
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl From<DbVec3> for na::Translation3<f32> {
    fn from(value: DbVec3) -> Self {
        na::Translation3::new(value.x, value.y, value.z)
    }
}

impl From<DbVec3> for na::Point3<f32> {
    fn from(v: DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}
impl From<&DbVec3> for na::Point3<f32> {
    fn from(v: &DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<DbVec3> for na::Vector3<f32> {
    fn from(v: DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}
impl From<&DbVec3> for na::Vector3<f32> {
    fn from(v: &DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<na::Vector3<f32>> for DbVec3 {
    fn from(v: na::Vector3<f32>) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<na::Point3<f32>> for DbVec3 {
    fn from(p: na::Point3<f32>) -> Self {
        DbVec3::new(p.x, p.y, p.z)
    }
}

impl From<DbVec3> for rapier3d::math::Isometry<f32> {
    fn from(v: DbVec3) -> Self {
        Self::translation(v.x, v.y, v.z)
    }
}
/// A unit quaternion (w + xi + yj + zk), stored as four `f32` scalars.
///
/// Semantics:
/// - Represents an orientation in world space.
/// - Stored in `(x, y, z, w)` order to match common game engine conventions.
/// - This is a purely data/serialization type; math happens elsewhere.
#[derive(SpacetimeType, Clone, Copy, PartialEq)]
pub struct DbQuat {
    /// x component (imaginary i)
    pub x: f32,
    /// y component (imaginary j)
    pub y: f32,
    /// z component (imaginary k)
    pub z: f32,
    /// w component (real part)
    pub w: f32,
}
impl DbQuat {
    fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl Default for DbQuat {
    /// Default orientation used by the server (matches Bevy's default)
    fn default() -> Self {
        Self::new(0.0, -1.0, 0.0, 0.0)
    }
}

impl From<DbQuat> for na::UnitQuaternion<f32> {
    fn from(q: DbQuat) -> Self {
        na::UnitQuaternion::from_quaternion(na::Quaternion::new(q.w, q.x, q.y, q.z))
    }
}

impl From<na::UnitQuaternion<f32>> for DbQuat {
    fn from(uq: na::UnitQuaternion<f32>) -> Self {
        let q = uq.into_inner();
        DbQuat {
            x: q.i,
            y: q.j,
            z: q.k,
            w: q.w,
        }
    }
}

/// Capsule dimensions for collider definitions.
///
/// Semantics:
/// - `radius`: radius of spherical caps and cylinder.
/// - `half_height`: half of the cylinder length along local +Y.
/// - Total capsule height = `2*half_height + 2*radius`.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
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
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
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
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbCone {
    pub radius: f32,
    pub half_height: f32,
}

/// Rounded-box parameters.
///
/// Semantics:
/// - `half_extents`: half extents of the cuboid (hx, hy, hz).
/// - `border_radius`: rounding radius applied to edges/corners.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct DbRoundCuboid {
    pub half_extents: DbVec3,
    pub border_radius: f32,
}

/// Rounded cylinder parameters (Y-aligned).
///
/// Semantics:
/// - `radius`: radius of the cylinder.
/// - `half_height`: half of the cylinder height along +Y.
/// - `border_radius`: rounding radius applied to edges/caps.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
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
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
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
#[derive(SpacetimeType, PartialEq)]
pub enum ColliderShape {
    /// Infinite plane (half-space). `f32` is the offset along the plane normal:
    /// the plane satisfies `n ⋅ x = dist`, where `n = rotation * +Y`.
    Plane(f32),
    /// Oriented box defined by local half-extents (hx, hy, hz).
    /// The final physics size used by the server is `half_extents * scale`.
    Cuboid(DbVec3),
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

/// Movement intent for an actor.
///
/// Match arms are handled by the server's tick reducer; unsupported variants
/// can be extended in the future.
#[derive(SpacetimeType, Debug, Clone, PartialEq)]
pub enum MoveIntent {
    /// Follow a sequence of waypoints (in world space) across multiple frames.
    Path(Vec<DbVec3>),

    /// Follow a dynamic actor by id.
    Actor(u64),

    /// Move toward this point (direction) for a single frame.
    Point(DbVec3),

    /// No movement intent (idling).
    None,
}

/// Logical kind/ownership for an actor.
///
/// Extend as needed for NPCs, bosses, and other categories.
#[derive(SpacetimeType, Debug, Clone, PartialEq)]
pub enum ActorKind {
    /// A player-controlled actor keyed by the user's identity.
    Player(Identity),
    /// A simple monster/NPC variant.
    Monster(u32),
}

#[derive(SpacetimeType)]
pub struct AoiActor {
    pub id: u64,
    pub transform_data_id: u64,
    pub identity: Option<Identity>,
    pub is_player: bool,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
    pub move_intent: MoveIntent,
}
impl From<Actor> for AoiActor {
    fn from(actor: Actor) -> Self {
        Self {
            id: actor.id,
            transform_data_id: actor.transform_data_id,
            identity: actor.identity,
            is_player: actor.is_player,
            capsule_radius: actor.capsule_radius,
            capsule_half_height: actor.capsule_half_height,
            move_intent: actor.move_intent,
        }
    }
}
