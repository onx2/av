#![allow(dead_code)]
//! Model utilities: lightweight conversions between database types and nalgebra,
//! plus a few small math helpers used by reducers and controller code.
//!
//! Design notes
//! - The database (schema.rs) defines simple, serializable types (DbVec3, DbQuat, …).
//! - This module provides ergonomic conversions to math types from `nalgebra`,
//!   without leaking math concerns into the schema.
//! - We avoid orphan-rule issues by implementing `From<T>` for our local types
//!   (e.g., `impl From<na::Vector3<f32>> for DbVec3`) and exposing free functions
//!   for the reverse direction (e.g., `vec3_from_db(DbVec3) -> na::Vector3<f32>`).

use nalgebra as na;

use crate::schema::{DbQuat, DbVec3};

/// Convert a database vector to nalgebra's `Vector3<f32>`.
///
/// This is the canonical way to get a math vector from a `DbVec3`.
#[inline]
pub fn vec3_from_db(v: DbVec3) -> na::Vector3<f32> {
    na::Vector3::new(v.x, v.y, v.z)
}

/// Convert a database vector to nalgebra's `Point3<f32>`.
///
/// Useful when you need a position type (affine point) for parry3d queries.
#[inline]
pub fn point3_from_db(v: DbVec3) -> na::Point3<f32> {
    na::Point3::new(v.x, v.y, v.z)
}

/// Convert a database quaternion to nalgebra's `UnitQuaternion<f32>`.
///
/// DbQuat is stored as `(x, y, z, w)`. nalgebra's `Quaternion::new` expects `(w, x, y, z)`.
#[inline]
pub fn unit_quat_from_db(q: DbQuat) -> na::UnitQuaternion<f32> {
    na::UnitQuaternion::from_quaternion(na::Quaternion::new(q.w, q.x, q.y, q.z))
}

/// Build an isometry (rigid transform) from DB translation and rotation.
///
/// This is the typical pose type used by parry3d narrow-phase queries.
#[inline]
pub fn iso_from_db(translation: DbVec3, rotation: DbQuat) -> na::Isometry3<f32> {
    na::Isometry3::from_parts(
        na::Translation3::new(translation.x, translation.y, translation.z),
        unit_quat_from_db(rotation),
    )
}

/// Create a pure yaw (rotation around world Y) as a DbQuat.
///
/// Theta is in radians. This uses the right-handed convention with Y up.
#[inline]
pub fn yaw_to_db_quat(theta: f32) -> DbQuat {
    // Construct a UnitQuaternion from yaw, then convert to DbQuat via `From`.
    let uq = na::UnitQuaternion::from_axis_angle(&na::Vector3::y_axis(), theta);
    DbQuat::from(uq)
}

/// Convert nalgebra's `Vector3<f32>` into a database vector.
///
/// Implemented as `From` on the local type to satisfy Rust orphan rules.
impl From<na::Vector3<f32>> for DbVec3 {
    #[inline]
    fn from(v: na::Vector3<f32>) -> Self {
        DbVec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        }
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
        let q = uq.into_inner(); // nalgebra::Quaternion { w, i, j, k }
        DbQuat {
            x: q.i,
            y: q.j,
            z: q.k,
            w: q.w,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_vec3() {
        let db = DbVec3 {
            x: 1.0,
            y: -2.0,
            z: 3.5,
        };
        let na_v = vec3_from_db(db);
        let db2 = DbVec3::from(na_v);
        assert_eq!(db.x, db2.x);
        assert_eq!(db.y, db2.y);
        assert_eq!(db.z, db2.z);
    }

    #[test]
    fn roundtrip_quat() {
        let yaw = 1.2345_f32;
        let db_q = yaw_to_db_quat(yaw);
        let na_uq = unit_quat_from_db(db_q);
        let db_q2 = DbQuat::from(na_uq);
        // Quaternions can differ by sign (q == -q), but this construction is consistent.
        assert!((db_q.x - db_q2.x).abs() < 1e-6);
        assert!((db_q.y - db_q2.y).abs() < 1e-6);
        assert!((db_q.z - db_q2.z).abs() < 1e-6);
        assert!((db_q.w - db_q2.w).abs() < 1e-6);
    }

    #[test]
    fn iso_from_db_constructs_proper_pose() {
        let t = DbVec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let q = yaw_to_db_quat(std::f32::consts::FRAC_PI_2);
        let iso = iso_from_db(t, q);
        assert!((iso.translation.vector.x - 1.0).abs() < 1e-6);
        assert!((iso.translation.vector.y - 2.0).abs() < 1e-6);
        assert!((iso.translation.vector.z - 3.0).abs() < 1e-6);
    }
}
