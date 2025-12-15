use nalgebra as na;

/// Default server-side maximum allowed movement intent distance (meters).
pub const MAX_INTENT_DISTANCE_SQ: f32 = 100.0 * 100.0;

/// Minimum planar motion required to update yaw (meters per tick).
pub const YAW_EPS: f32 = 1.0e-6;

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

impl UtilMath for f32 {
    fn sq(self) -> Self {
        self * self
    }
}

/// Planar (XZ) distance squared between two world positions (meters^2).
pub fn planar_distance_sq(a: na::Vector3<f32>, b: na::Vector3<f32>) -> f32 {
    (b.x - a.x).sq() + (b.z - a.z).sq()
}

/// Are two positions within a planar movement range (meters)?
pub fn within_movement_range(a: na::Vector3<f32>, b: na::Vector3<f32>) -> bool {
    planar_distance_sq(a, b) <= MAX_INTENT_DISTANCE_SQ
}

/// Are two positions within a planar acceptance radius (meters)?
pub fn within_acceptance(a: na::Vector3<f32>, b: na::Vector3<f32>, acceptance_radius: f32) -> bool {
    if acceptance_radius < 0.0 {
        return true;
    }
    planar_distance_sq(a, b) <= acceptance_radius.sq()
}
