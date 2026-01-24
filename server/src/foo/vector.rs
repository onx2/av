use nalgebra::Vector3;
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Debug, Default, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
}

#[derive(SpacetimeType, Debug, Default, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn xz(&self) -> Vec2 {
        Vec2::new(self.x, self.z)
    }
}

impl From<Vector3<f32>> for Vec3 {
    fn from(v: Vector3<f32>) -> Self {
        Vec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}
impl From<Vec3> for Vector3<f32> {
    fn from(v: Vec3) -> Self {
        Vector3::new(v.x, v.y, v.z)
    }
}
