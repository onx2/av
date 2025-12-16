use crate::module_bindings::{DbQuat, DbVec3};
use bevy::prelude::*;
use nalgebra as na;

impl From<DbVec3> for Vec3 {
    fn from(vec3: DbVec3) -> Self {
        Vec3 {
            x: vec3.x,
            y: vec3.y,
            z: vec3.z,
        }
    }
}

impl From<Vec3> for DbVec3 {
    fn from(vec3: Vec3) -> Self {
        DbVec3 {
            x: vec3.x,
            y: vec3.y,
            z: vec3.z,
        }
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
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}
impl From<&na::Vector3<f32>> for DbVec3 {
    fn from(v: &na::Vector3<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<DbQuat> for Quat {
    fn from(quat: DbQuat) -> Self {
        Quat::from_array([quat.x, quat.y, quat.z, quat.w])
    }
}

impl From<Quat> for DbQuat {
    fn from(quat: Quat) -> Self {
        DbQuat {
            x: quat.x,
            y: quat.y,
            z: quat.z,
            w: quat.w,
        }
    }
}
