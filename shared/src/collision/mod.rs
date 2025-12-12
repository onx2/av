/*!
Shared collision types and a kinematic sweep-and-slide skeleton.

Goal:
- Define shape/transform structs used by both server and client.
- Provide a kinematic capsule movement function that will later use parry3d's TOI queries.

Notes:
- This is a skeleton: the parry3d shape-casting is stubbed out for now.
- The sweep-and-slide loop is implemented and will move without collisions until the cast function is completed.
*/

use nalgebra as na;
type Vec3 = na::Vector3<f32>;
type Quat = na::UnitQuaternion<f32>;

// Keep the deps visible so the crate links them successfully.
// We'll wire the real queries next pass.
// nalgebra imported above
#[allow(unused_imports)]
use parry3d::{self as parry, shape as pshape};
pub mod broad;

/// A rigid transform used by statics (pose in world space).
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat, // isometry = (translation, rotation)
}

/// Static collision shapes supported by the world.
#[derive(Clone, Copy, Debug)]
pub enum StaticShape {
    /// Plane equation: normal · x = dist (in world space).
    Plane { normal: Vec3, dist: f32 },

    /// Oriented cuboid with the given half extents and pose.
    /// The half extents are in the cuboid's local space.
    Cuboid {
        half_extents: Vec3,
        transform: Transform,
    },
}

/// Capsule spec for "actors".
/// half_height is along the actor's local +Y (i.e., capsule segment extends ±half_height on Y).
#[derive(Clone, Copy, Debug)]
pub struct CapsuleSpec {
    pub radius: f32,
    pub half_height: f32,
}

/// Parameters for a single kinematic movement attempt.
#[derive(Clone, Copy, Debug)]
pub struct MoveRequest {
    /// Starting world position of the capsule's center.
    pub start_pos: Vec3,
    /// Desired world-space translation for this step (e.g., from input/pathing).
    pub desired_translation: Vec3,
    /// Capsule shape for the actor.
    pub capsule: CapsuleSpec,
    /// Separation to keep from surfaces to avoid jitter.
    pub skin: f32,
    /// Max iterations of slide resolution (for corners).
    pub max_iterations: u32,
}

/// Details about the most recent hit during the move (if any).
#[derive(Clone, Copy, Debug)]
pub struct MoveHit {
    /// Surface normal at the impact.
    pub normal: Vec3,
    /// Fraction [0, 1] of the requested translation distance traveled before impact.
    pub fraction: f32,
}

/// Result of the kinematic move.
#[derive(Clone, Copy, Debug)]
pub struct MoveResult {
    /// Final world position of the capsule center after sweep-and-slide.
    pub end_pos: Vec3,
    /// If there was a collision on the last iteration, details about it.
    pub last_hit: Option<MoveHit>,
    /// Any remaining translation that wasn't consumed (usually zero if we ended early).
    pub remaining: Vec3,
}

// removed: to_na_iso helper (Vec3/Quat are nalgebra types already)

// removed: to_na_vec3 helper (Vec3 is already nalgebra::Vector3<f32>)

/// Attempt to cast a moving capsule against a single static shape and return the earliest hit (if any).
///
/// TODO: Implement with parry3d's query functions (e.g., `time_of_impact` / `cast_shape`) for:
/// - Capsule vs HalfSpace (Plane)
/// - Capsule vs Cuboid
fn cast_capsule_against_static(
    capsule_iso: na::Isometry3<f32>,
    capsule: &pshape::Capsule,
    vel: na::Vector3<f32>,
    max_toi: f32,
    static_shape: &StaticShape,
) -> Option<MoveHit> {
    match *static_shape {
        StaticShape::Plane { normal, dist } => {
            // Build a HalfSpace whose plane is: normal · x = dist in world space.
            // We encode this as a halfspace with the given world normal, positioned at normal * dist.
            let unit_n = na::Unit::new_normalize(normal);
            let plane = pshape::HalfSpace { normal: unit_n };
            let plane_iso = na::Isometry3::from_parts(
                na::Translation3::new((normal * dist).x, (normal * dist).y, (normal * dist).z),
                Quat::identity(),
            );

            if let Ok(Some(toi)) = parry::query::time_of_impact(
                &capsule_iso,
                &vel,
                capsule,
                &plane_iso,
                &na::Vector3::zeros(),
                &plane,
                max_toi,
                true,
            ) {
                // Use the normal on the moving shape; flip to oppose motion if needed.
                let v = vel;
                let n_na = toi.normal1.into_inner();
                let mut n = Vec3::new(n_na.x, n_na.y, n_na.z);
                if n.dot(&v) > 0.0 {
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
            let he = half_extents;
            let cuboid = pshape::Cuboid::new(he);
            let box_iso = na::Isometry3::from_parts(
                na::Translation3::new(
                    transform.translation.x,
                    transform.translation.y,
                    transform.translation.z,
                ),
                transform.rotation,
            );

            if let Ok(Some(toi)) = parry::query::time_of_impact(
                &capsule_iso,
                &vel,
                capsule,
                &box_iso,
                &na::Vector3::zeros(),
                &cuboid,
                max_toi,
                true,
            ) {
                let v = vel;
                let n_na = toi.normal1.into_inner();
                let mut n = Vec3::new(n_na.x, n_na.y, n_na.z);
                if n.dot(&v) > 0.0 {
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

/// Kinematic sweep-and-slide for a capsule against a set of static shapes.
/// - Performs a shape cast of the capsule along the desired translation.
/// - On hit, moves to impact point minus `skin`, then slides along the surface normal.
/// - Iterates up to `max_iterations` times to handle corners.
pub fn move_capsule_kinematic(statics: &[StaticShape], req: MoveRequest) -> MoveResult {
    let mut pos = req.start_pos;
    let mut remaining = req.desired_translation;
    let mut last_hit = None;

    // The actor capsule is aligned with world +Y (rotate capsule by identity).
    let capsule_shape = pshape::Capsule::new_y(req.capsule.half_height, req.capsule.radius);

    for _ in 0..req.max_iterations {
        let len_sq = remaining.norm_squared();
        if len_sq <= 1.0e-10 {
            break;
        }
        let len = remaining.norm();
        let dir = remaining / len;

        let capsule_iso =
            na::Isometry3::from_parts(na::Translation3::new(pos.x, pos.y, pos.z), Quat::identity());
        let vel = dir * len;

        // Find the earliest hit across all statics.
        let mut best: Option<MoveHit> = None;
        for s in statics {
            if let Some(hit) = cast_capsule_against_static(capsule_iso, &capsule_shape, vel, 1.0, s)
            {
                if best.map_or(true, |b| hit.fraction < b.fraction) {
                    best = Some(hit);
                }
            }
        }

        match best {
            None => {
                // No hit → move fully and finish.
                pos += remaining;
                remaining = na::Vector3::zeros();
                last_hit = None;
                break;
            }
            Some(hit) => {
                // Travel up to the contact point (minus skin).
                let travel = (len * hit.fraction).max(0.0);
                let advance = dir * (travel - req.skin).max(0.0);
                pos += advance;

                // Slide along the hit plane: remove the normal component from the leftover.
                let n = if hit.normal.norm_squared() > 1.0e-12 {
                    hit.normal / hit.normal.norm()
                } else {
                    na::Vector3::zeros()
                };
                let leftover = dir * (len - travel);
                let slide = leftover - n * leftover.dot(&n);

                remaining = slide;
                last_hit = Some(hit);

                // If the slide is negligible, we're done.
                if slide.norm_squared() <= 1.0e-10 {
                    break;
                }
            }
        }
    }

    MoveResult {
        end_pos: pos,
        last_hit,
        remaining,
    }
}

/// Keep a capsule hovering above the nearest ground within `max_snap_distance`.
/// - `pos` is the current capsule center (world space).
/// - If a hit is found within the downward cast range, the capsule center is moved to the
///   impact position, offset by `hover_height` along the surface normal.
/// - If no hit is found, returns `pos` unchanged.
pub fn snap_capsule_to_ground(
    statics: &[StaticShape],
    capsule: CapsuleSpec,
    pos: Vec3,
    max_snap_distance: f32,
    hover_height: f32,
) -> (Vec3, bool) {
    if max_snap_distance <= 0.0 || hover_height < 0.0 {
        return (pos, false);
    }

    let capsule_shape = pshape::Capsule::new_y(capsule.half_height, capsule.radius);
    let capsule_iso =
        na::Isometry3::from_parts(na::Translation3::new(pos.x, pos.y, pos.z), Quat::identity());
    let vel = na::Vector3::new(0.0, -max_snap_distance, 0.0);

    // Find earliest downward hit.
    let mut best: Option<MoveHit> = None;
    for s in statics {
        if let Some(hit) = cast_capsule_against_static(capsule_iso, &capsule_shape, vel, 1.0, s) {
            if best.as_ref().map_or(true, |b| hit.fraction < b.fraction) {
                best = Some(hit);
            }
        }
    }

    if let Some(hit) = best {
        // Impact center (fraction of the cast).
        let impact_center = pos + vel * hit.fraction;

        // Ensure the offset normal opposes motion (consistent with slide code).
        let mut n = hit.normal;
        if n.dot(&vel) > 0.0 {
            n = -n;
        }

        // Hover slightly above ground along the contact normal.
        let new_pos = impact_center + n * hover_height.max(0.0);
        return (new_pos, true);
    }

    (pos, false)
}

/// Convenience: build a `StaticShape::Plane` from a world-space plane pose:
/// - normal = rotation * +Y
/// - dist = dot(normal, translation) + optional offset
pub fn plane_from_pose(rotation: Quat, translation: Vec3, offset_along_normal: f32) -> StaticShape {
    let normal = rotation * Vec3::new(0.0, 1.0, 0.0);
    let dist = normal.dot(&translation) + offset_along_normal;
    StaticShape::Plane { normal, dist }
}

/// Convenience: build a `StaticShape::Cuboid` with given half extents and pose.
pub fn cuboid_from_pose(half_extents: Vec3, translation: Vec3, rotation: Quat) -> StaticShape {
    StaticShape::Cuboid {
        half_extents,
        transform: Transform {
            translation,
            rotation,
        },
    }
}

/// Broad-phase accelerated variant of `move_capsule_kinematic`.
/// Uses a prebuilt accelerator to prune candidate statics before running the narrow-phase sweep.
pub fn move_capsule_kinematic_with_accel(
    statics: &[StaticShape],
    accel: &broad::WorldAccel,
    req: MoveRequest,
) -> MoveResult {
    // Compute the swept AABB for this move.
    let swept = broad::swept_capsule_aabb(
        req.capsule.half_height,
        req.capsule.radius,
        req.start_pos,
        req.desired_translation,
        req.skin,
    );

    // Build a small subset of statics to test: all planes + AABB-overlapping finite shapes.
    let mut subset: Vec<StaticShape> = Vec::new();
    for &idx in &accel.plane_indices {
        subset.push(statics[idx]);
    }
    for idx in broad::query_candidates(accel, &swept) {
        subset.push(statics[idx]);
    }

    // Delegate to the narrow-phase sweep-and-slide on the pruned set.
    move_capsule_kinematic(&subset, req)
}

/// Broad-phase accelerated ground-snap. Queries only candidates from the accelerator plus planes.
pub fn snap_capsule_to_ground_with_accel(
    statics: &[StaticShape],
    accel: &broad::WorldAccel,
    capsule: CapsuleSpec,
    pos: Vec3,
    max_snap_distance: f32,
    hover_height: f32,
) -> (Vec3, bool) {
    let desired = na::Vector3::new(0.0, -max_snap_distance, 0.0);
    let swept = broad::swept_capsule_aabb(capsule.half_height, capsule.radius, pos, desired, 0.0);

    // Subset = planes + finite shapes overlapping the down-sweep.
    let mut subset: Vec<StaticShape> = Vec::new();
    for &idx in &accel.plane_indices {
        subset.push(statics[idx]);
    }
    for idx in broad::query_candidates(accel, &swept) {
        subset.push(statics[idx]);
    }

    snap_capsule_to_ground(&subset, capsule, pos, max_snap_distance, hover_height)
}
