use nalgebra as na;
use parry3d::{
    query::{self, ShapeCastOptions},
    shape as pshape,
};

use super::types::{Iso, MoveHit, StaticShape, Vec3};
use parry3d::shape::Shape as _;

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

            // parry3d 0.25: use builder to set maximum time of impact and pass options by value.
            let mut opts = ShapeCastOptions::with_max_time_of_impact(max_toi);
            opts.stop_at_penetration = true;
            if let Ok(Some(hit)) = query::cast_shapes(
                &capsule_iso,
                &vel,
                capsule as &dyn pshape::Shape,
                &plane_iso,
                &na::Vector3::zeros(),
                &plane as &dyn pshape::Shape,
                opts,
            ) {
                // Use the normal on the moving shape; ensure it opposes the motion.
                let mut n = Vec3::new(
                    hit.normal1.into_inner().x,
                    hit.normal1.into_inner().y,
                    hit.normal1.into_inner().z,
                );
                if n.dot(&vel) > 0.0 {
                    n = -n;
                }
                return Some(MoveHit {
                    normal: n,
                    fraction: hit.time_of_impact,
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

            let mut opts = ShapeCastOptions::with_max_time_of_impact(max_toi);
            opts.stop_at_penetration = true;
            if let Ok(Some(hit)) = query::cast_shapes(
                &capsule_iso,
                &vel,
                capsule as &dyn pshape::Shape,
                &box_iso,
                &na::Vector3::zeros(),
                &cuboid as &dyn pshape::Shape,
                opts,
            ) {
                let mut n = Vec3::new(
                    hit.normal1.into_inner().x,
                    hit.normal1.into_inner().y,
                    hit.normal1.into_inner().z,
                );
                if n.dot(&vel) > 0.0 {
                    n = -n;
                }
                return Some(MoveHit {
                    normal: n,
                    fraction: hit.time_of_impact,
                });
            }
            None
        }
        StaticShape::Sphere { radius, transform } => {
            // Treat as a Ball; rotation is irrelevant.
            let ball = pshape::Ball::new(radius);
            let ball_iso = transform.iso();

            let mut opts = ShapeCastOptions::with_max_time_of_impact(max_toi);
            opts.stop_at_penetration = true;
            if let Ok(Some(hit)) = query::cast_shapes(
                &capsule_iso,
                &vel,
                capsule as &dyn pshape::Shape,
                &ball_iso,
                &na::Vector3::zeros(),
                &ball as &dyn pshape::Shape,
                opts,
            ) {
                let mut normal = Vec3::new(
                    hit.normal1.into_inner().x,
                    hit.normal1.into_inner().y,
                    hit.normal1.into_inner().z,
                );
                if normal.dot(&vel) > 0.0 {
                    normal = -normal;
                }
                return Some(MoveHit {
                    normal,
                    fraction: hit.time_of_impact,
                });
            }
            None
        }
        StaticShape::Capsule {
            radius,
            half_height,
            transform,
        } => {
            // Static capsule vs moving capsule.
            let static_capsule = pshape::Capsule::new_y(half_height, radius);
            let static_iso = transform.iso();

            let mut opts = ShapeCastOptions::with_max_time_of_impact(max_toi);
            opts.stop_at_penetration = true;
            if let Ok(Some(hit)) = query::cast_shapes(
                &capsule_iso,
                &vel,
                capsule as &dyn pshape::Shape,
                &static_iso,
                &na::Vector3::zeros(),
                &static_capsule as &dyn pshape::Shape,
                opts,
            ) {
                let mut n = Vec3::new(
                    hit.normal1.into_inner().x,
                    hit.normal1.into_inner().y,
                    hit.normal1.into_inner().z,
                );
                if n.dot(&vel) > 0.0 {
                    n = -n;
                }
                return Some(MoveHit {
                    normal: n,
                    fraction: hit.time_of_impact,
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
