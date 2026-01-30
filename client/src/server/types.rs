use crate::module_bindings;
use bevy::prelude::*;
use nalgebra as na;

impl From<module_bindings::Vec3> for Vec3 {
    fn from(vec3: module_bindings::Vec3) -> Self {
        Vec3 {
            x: vec3.x,
            y: vec3.y,
            z: vec3.z,
        }
    }
}

impl From<Vec3> for module_bindings::Vec3 {
    fn from(vec3: Vec3) -> Self {
        module_bindings::Vec3 {
            x: vec3.x,
            y: vec3.y,
            z: vec3.z,
        }
    }
}

impl From<module_bindings::Vec3> for na::Vector3<f32> {
    fn from(v: module_bindings::Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}
impl From<&module_bindings::Vec3> for na::Vector3<f32> {
    fn from(v: &module_bindings::Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<na::Vector3<f32>> for module_bindings::Vec3 {
    fn from(v: na::Vector3<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}
impl From<&na::Vector3<f32>> for module_bindings::Vec3 {
    fn from(v: &na::Vector3<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<module_bindings::Quat> for Quat {
    fn from(quat: module_bindings::Quat) -> Self {
        Quat::from_array([quat.x, quat.y, quat.z, quat.w])
    }
}

impl From<Quat> for module_bindings::Quat {
    fn from(quat: Quat) -> Self {
        module_bindings::Quat {
            x: quat.x,
            y: quat.y,
            z: quat.z,
            w: quat.w,
        }
    }
}
