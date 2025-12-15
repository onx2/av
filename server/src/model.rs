use nalgebra as na;
use spacetimedb::ScheduleAt;

use crate::schema::{DbQuat, DbVec3};

/// Planar (XZ) distance squared between two world positions (meters^2).
pub fn planar_distance_sq(a: DbVec3, b: DbVec3) -> f32 {
    let dx = b.x - a.x;
    let dz = b.z - a.z;
    dx * dx + dz * dz
}

/// Are two positions within a planar movement range (meters)?
pub fn within_movement_range(a: DbVec3, b: DbVec3, max_distance: f32) -> bool {
    if max_distance <= 0.0 {
        return false;
    }
    planar_distance_sq(a, b) <= max_distance * max_distance
}

/// Are two positions within a planar acceptance radius (meters)?
pub fn within_acceptance(a: DbVec3, b: DbVec3, acceptance_radius: f32) -> bool {
    if acceptance_radius < 0.0 {
        return true;
    }
    planar_distance_sq(a, b) <= acceptance_radius * acceptance_radius
}

/// Default server-side maximum allowed movement intent distance (meters).
pub const DEFAULT_MAX_INTENT_DISTANCE: f32 = 100.0;

pub fn get_variable_delta_time(
    now: spacetimedb::Timestamp,
    last: spacetimedb::Timestamp,
) -> Option<f32> {
    now.time_duration_since(last)
        .map(|dur| dur.to_micros() as f32 / 1_000_000.0)
}

pub fn get_fixed_delta_time(scheduled_at: ScheduleAt) -> f32 {
    match scheduled_at {
        ScheduleAt::Interval(dt) => dt.to_micros() as f32 / 1_000_000.0,
        _ => panic!("Expected ScheduleAt to be Interval"),
    }
}

/// Convert a database vector to nalgebra's `Vector3<f32>`.
///
/// This is the canonical way to get a math vector from a `DbVec3`.
pub fn vec3_from_db(v: DbVec3) -> na::Vector3<f32> {
    na::Vector3::new(v.x, v.y, v.z)
}

/// Convert a database vector to nalgebra's `Point3<f32>`.
///
/// Useful when you need a position type (affine point) for parry3d queries.
pub fn point3_from_db(v: DbVec3) -> na::Point3<f32> {
    na::Point3::new(v.x, v.y, v.z)
}

/// Convert a database quaternion to nalgebra's `UnitQuaternion<f32>`.
///
/// DbQuat is stored as `(x, y, z, w)`. nalgebra's `Quaternion::new` expects `(w, x, y, z)`.
pub fn unit_quat_from_db(q: DbQuat) -> na::UnitQuaternion<f32> {
    na::UnitQuaternion::from_quaternion(na::Quaternion::new(q.w, q.x, q.y, q.z))
}

/// Build an isometry (rigid transform) from DB translation and rotation.
///
/// This is the typical pose type used by parry3d narrow-phase queries.
pub fn iso_from_db(translation: DbVec3, rotation: DbQuat) -> na::Isometry3<f32> {
    na::Isometry3::from_parts(
        na::Translation3::new(translation.x, translation.y, translation.z),
        unit_quat_from_db(rotation),
    )
}

/// Create a pure yaw (rotation around world Y) as a DbQuat.
///
/// Theta is in radians. This uses the right-handed convention with Y up.
pub fn yaw_to_db_quat(theta: f32) -> DbQuat {
    // Construct a UnitQuaternion from yaw, then convert to DbQuat via `From`.
    let uq = na::UnitQuaternion::from_axis_angle(&na::Vector3::y_axis(), theta);
    DbQuat::from(uq)
}

/// Convert nalgebra's `Vector3<f32>` into a database vector.
///
/// Implemented as `From` on the local type to satisfy Rust orphan rules.
impl From<na::Vector3<f32>> for DbVec3 {
    fn from(v: na::Vector3<f32>) -> Self {
        DbVec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<DbVec3> for na::Vector3<f32> {
    fn from(v: DbVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

/// Convert nalgebra's `Point3<f32>` into a database vector (dropping the affine distinction).
///
/// This is a convenience for persisting positions back into the DB.
impl From<na::Point3<f32>> for DbVec3 {
    #[inline]
    fn from(p: na::Point3<f32>) -> Self {
        DbVec3 {
            x: p.x,
            y: p.y,
            z: p.z,
        }
    }
}

/// Convert nalgebra's `UnitQuaternion<f32>` into a database quaternion.
///
/// Stored as `(x, y, z, w)` which matches common engine conventions.
impl From<na::UnitQuaternion<f32>> for DbQuat {
    #[inline]
    fn from(uq: na::UnitQuaternion<f32>) -> Self {
        // nalgebra::Quaternion { w, i, j, k }
        let q = uq.into_inner();
        DbQuat {
            x: q.i,
            y: q.j,
            z: q.k,
            w: q.w,
        }
    }
}
