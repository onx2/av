use crate::module_bindings::{DbQuat, DbVec3};
use bevy::prelude::*;

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
