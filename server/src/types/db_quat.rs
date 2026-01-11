/// A unit quaternion (w + xi + yj + zk), stored as four `f32` scalars.
///
/// Semantics:
/// - Represents an orientation in world space.
/// - Stored in `(x, y, z, w)` order to match common game engine conventions.
/// - This is a purely data/serialization type; math happens elsewhere.
#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq)]
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

impl From<DbQuat> for nalgebra::UnitQuaternion<f32> {
    fn from(q: DbQuat) -> Self {
        nalgebra::UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(q.w, q.x, q.y, q.z))
    }
}

impl From<nalgebra::UnitQuaternion<f32>> for DbQuat {
    fn from(uq: nalgebra::UnitQuaternion<f32>) -> Self {
        let q = uq.into_inner();
        DbQuat {
            x: q.i,
            y: q.j,
            z: q.k,
            w: q.w,
        }
    }
}
