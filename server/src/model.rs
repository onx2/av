use nalgebra as na;
use spacetimedb::SpacetimeType;

/// A 3D vector in world space (meters).
///
/// Semantics:
/// - Used for translations, scales, and general scalar triplets.
/// - This is a data type only; math/conversions live outside the schema.
#[derive(SpacetimeType, Clone, Copy, PartialEq)]
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

impl From<DbVec3> for na::Vector3<f32> {
    fn from(v: DbVec3) -> Self {
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
