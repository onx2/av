use crate::module_bindings::{DbQuat, DbVec3, DbVec3I16};
use bevy::prelude::*;
use nalgebra as na;
use shared::constants::Y_QUANTIZE_STEP_M;

/// Mixed-precision translation convention (matches the server schema):
/// - `x`/`z` are meters stored as `f32` (full precision)
/// - `y` is quantized `i16` in `Y_QUANTIZE_STEP_M` meter units
///
/// Note: `DbVec3I16` is generated; confirm its field types whenever you re-generate bindings.

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

/// Decode mixed-precision `DbVec3I16` into a Bevy `Vec3` in meters.
///
/// Convention:
/// - `x`/`z` are already meters (`f32`)
/// - `y` is quantized `i16` in `Y_QUANTIZE_STEP_M` meter units
impl From<DbVec3I16> for Vec3 {
    fn from(v: DbVec3I16) -> Self {
        Vec3::new(v.x, v.y as f32 * Y_QUANTIZE_STEP_M, v.z)
    }
}

/// Decode mixed-precision `DbVec3I16` into a nalgebra `Vector3<f32>` in meters.
impl From<DbVec3I16> for na::Vector3<f32> {
    fn from(v: DbVec3I16) -> Self {
        na::Vector3::new(v.x, v.y as f32 * Y_QUANTIZE_STEP_M, v.z)
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
