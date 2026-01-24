use nalgebra::{Quaternion, UnitQuaternion};
use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Quat { x, y, z, w }
    }
}

impl From<Quat> for UnitQuaternion<f32> {
    fn from(q: Quat) -> Self {
        Self::from_quaternion(Quaternion::new(q.w, q.x, q.y, q.z))
    }
}

impl From<UnitQuaternion<f32>> for Quat {
    fn from(uq: UnitQuaternion<f32>) -> Self {
        let q = uq.into_inner();
        Self {
            x: q.i,
            y: q.j,
            z: q.k,
            w: q.w,
        }
    }
}
