use nalgebra::Vector2;

/// A 3D vector in world space (meters).
///
/// Semantics:
/// - Used for translations, scales, and general scalar triplets.
/// - This is a data type only; math/conversions live outside the schema.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq)]
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
    pub fn vec2_xz(self) -> Vector2<f32> {
        Vector2::new(self.x, self.z)
    }
}

impl From<DbVec3> for nalgebra::Translation3<f32> {
    fn from(value: DbVec3) -> Self {
        nalgebra::Translation3::new(value.x, value.y, value.z)
    }
}

impl From<DbVec3> for nalgebra::Point3<f32> {
    fn from(v: DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}
impl From<&DbVec3> for nalgebra::Point3<f32> {
    fn from(v: &DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<DbVec3> for nalgebra::Vector3<f32> {
    fn from(v: DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}
impl From<&DbVec3> for nalgebra::Vector3<f32> {
    fn from(v: &DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<nalgebra::Vector3<f32>> for DbVec3 {
    fn from(v: nalgebra::Vector3<f32>) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<nalgebra::Point3<f32>> for DbVec3 {
    fn from(p: nalgebra::Point3<f32>) -> Self {
        DbVec3::new(p.x, p.y, p.z)
    }
}

impl From<DbVec3> for rapier3d::math::Isometry<f32> {
    fn from(v: DbVec3) -> Self {
        Self::translation(v.x, v.y, v.z)
    }
}
