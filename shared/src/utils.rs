use super::constants::*;
use nalgebra as na;

pub fn yaw_from_xz(x: f32, z: f32) -> Option<f32> {
    if x * x + z * z > YAW_EPS {
        Some((-x).atan2(-z))
    } else {
        None
    }
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
pub fn is_move_too_far(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> bool {
    planar_distance_sq(a, b) > MAX_INTENT_DISTANCE_SQ
}

/// Are two positions within a planar acceptance radius (meters)?
pub fn is_move_too_close(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> bool {
    planar_distance_sq(a, b) <= SMALLEST_REQUEST_DISTANCE_SQ
}
