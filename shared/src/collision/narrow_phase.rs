use nalgebra as na;
use parry3d::{query, shape as pshape};

use super::types::{Iso, MoveHit, StaticShape, Vec3};

/// Cast a moving Y-aligned capsule against a single static shape and return the earliest hit (if any).
///
/// - `capsule_iso`: the capsule's starting isometry in world space.
/// - `capsule`: the capsule shape (Y-aligned) being swept.
/// - `vel`: the world-space translation vector for this cast (units: meters).
/// - `max_toi`: the maximum fraction of `vel` to consider (typically 1.0).
/// - `shape`: the static shape to test against.
///
/// Returns the impact normal (on the moving capsule) and the fraction along `vel` where the hit occurs.
pub fn cast_capsule_against_static(
    capsule_iso: Iso,
    capsule: &pshape::Capsule,
    vel: Vec3,
    max_toi: f32,
    shape: &StaticShape,
) -> Option<MoveHit> {
    match *shape {
        StaticShape::Plane { normal, dist } => {
            // Plane: represent as a parry HalfSpace with world normal, positioned at normal * dist.
            // Plane equation in world space: normal ⋅ x = dist
            let unit_n = na::Unit::new_normalize(normal);
            let plane = pshape::HalfSpace { normal: unit_n };
            let plane_iso = Iso::from_parts(
                na::Translation3::new((normal * dist).x, (normal * dist).y, (normal * dist).z),
                na::UnitQuaternion::identity(),
            );

            if let Ok(Some(toi)) = query::time_of_impact(
                &capsule_iso,
                &vel,
                capsule,
                &plane_iso,
                &na::Vector3::zeros(),
                &plane,
                max_toi,
                true,
            ) {
                // Use the normal on the moving shape; ensure it opposes the motion.
                let mut n = Vec3::new(
                    toi.normal1.into_inner().x,
                    toi.normal1.into_inner().y,
                    toi.normal1.into_inner().z,
                );
                if n.dot(&vel) > 0.0 {
                    n = -n;
                }
                return Some(MoveHit {
                    normal: n,
                    fraction: toi.toi,
                });
            }
            None
        }
        StaticShape::Cuboid {
            half_extents,
            transform,
        } => {
            let cuboid = pshape::Cuboid::new(half_extents);
            let box_iso = transform.iso();

            if let Ok(Some(toi)) = query::time_of_impact(
                &capsule_iso,
                &vel,
                capsule,
                &box_iso,
                &na::Vector3::zeros(),
                &cuboid,
                max_toi,
                true,
            ) {
                let mut n = Vec3::new(
                    toi.normal1.into_inner().x,
                    toi.normal1.into_inner().y,
                    toi.normal1.into_inner().z,
                );
                if n.dot(&vel) > 0.0 {
                    n = -n;
                }
                return Some(MoveHit {
                    normal: n,
                    fraction: toi.toi,
                });
            }
            None
        }
    }
}

/// Iterate over a list of static shapes and return the earliest capsule hit (if any).
///
/// This is a convenience wrapper that repeatedly calls [`cast_capsule_against_static`] and
/// selects the minimum time-of-impact across all shapes.
pub fn earliest_toi_capsule_vs_statics(
    capsule_iso: Iso,
    capsule: &pshape::Capsule,
    vel: Vec3,
    max_toi: f32,
    statics: &[StaticShape],
) -> Option<MoveHit> {
    let mut best: Option<MoveHit> = None;
    for s in statics {
        if let Some(hit) = cast_capsule_against_static(capsule_iso, capsule, vel, max_toi, s) {
            if best.as_ref().map_or(true, |b| hit.fraction < b.fraction) {
                best = Some(hit);
            }
        }
    }
    best
}
