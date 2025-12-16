use super::constants::*;
use nalgebra as na;

pub fn rotation_from_xz(x: f32, z: f32) -> Option<na::Unit<na::Quaternion<f32>>> {
    if x.sq() + z.sq() > YAW_EPS {
        Some(na::UnitQuaternion::from_axis_angle(
            &na::Vector3::y_axis(),
            yaw(x, z),
        ))
    } else {
        None
    }
}

pub fn yaw(x: f32, z: f32) -> f32 {
    (-x).atan2(-z)
}

pub trait UtilMath {
    fn sq(self) -> Self;
}

impl<T> UtilMath for T
where
    T: std::ops::Mul<Output = T> + Copy,
{
    fn sq(self) -> Self {
        self * self
    }
}

/// Planar (XZ) distance squared between two world positions (meters^2).
pub fn planar_distance_sq(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> f32 {
    (b.x - a.x).sq() + (b.z - a.z).sq()
}

/// Are two positions within a planar movement range (meters)?
pub fn within_movement_range(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> bool {
    planar_distance_sq(a, b) <= MAX_INTENT_DISTANCE_SQ
}

/// Are two positions within a planar acceptance radius (meters)?
pub fn within_movement_acceptance(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> bool {
    planar_distance_sq(a, b) <= SMALLEST_REQUEST_DISTANCE_SQ
}
