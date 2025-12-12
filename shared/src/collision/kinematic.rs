use nalgebra as na;
use parry3d::shape as pshape;

use super::{
    broad, narrow_phase,
    settings::{DEFAULT_MAX_ITERATIONS, DEFAULT_SKIN, MIN_MOVE_SQ},
    types::{CapsuleSpec, Iso, MoveHit, MoveResult, StaticShape, Vec3},
};

/// Parameters for a single kinematic movement attempt.
///
/// - Movement is expressed as a desired translation for this tick (meters).
/// - Collision is handled by shape-casting a Y-aligned capsule using parry3d's TOI query,
///   stopping at contact minus `skin`, and sliding along the contact normal.
/// - The slide step iterates up to `max_iterations` to handle corners.
#[derive(Clone, Copy, Debug)]
pub struct MoveRequest {
    /// Starting world position of the capsule's center.
    pub start_pos: Vec3,
    /// Desired world-space translation for this step (e.g., from input/pathing).
    pub desired_translation: Vec3,
    /// Capsule shape for the actor.
    pub capsule: CapsuleSpec,
    /// Separation to keep from surfaces to avoid jitter (meters).
    pub skin: f32,
    /// Max iterations of slide resolution (for corners).
    pub max_iterations: u32,
}

impl MoveRequest {
    #[inline]
    pub fn with_defaults(start_pos: Vec3, desired_translation: Vec3, capsule: CapsuleSpec) -> Self {
        Self {
            start_pos,
            desired_translation,
            capsule,
            skin: DEFAULT_SKIN,
            max_iterations: DEFAULT_MAX_ITERATIONS,
        }
    }
}

/// Kinematic sweep-and-slide for a capsule against a set of static shapes.
///
/// Algorithm:
/// - Shape-cast the capsule (TOI) along the desired translation.
/// - On hit, move to just before the contact (minus `skin`) and slide along the contact plane.
/// - Iterate to handle corners until `max_iterations` or the remaining motion is negligible.
pub fn move_capsule_kinematic(statics: &[StaticShape], req: MoveRequest) -> MoveResult {
    let mut pos = req.start_pos;
    let mut remaining = req.desired_translation;
    let mut last_hit = None;

    // Y-aligned capsule (controller axis is +Y).
    let capsule_shape = pshape::Capsule::new_y(req.capsule.half_height, req.capsule.radius);

    for _ in 0..req.max_iterations {
        // Early out if remaining motion is too small to matter.
        if remaining.norm_squared() <= MIN_MOVE_SQ {
            break;
        }

        let len = remaining.norm();
        let dir = remaining / len;

        let capsule_iso: Iso = Iso::from_parts(
            na::Translation3::new(pos.x, pos.y, pos.z),
            na::UnitQuaternion::identity(),
        );
        let vel = dir * len;

        // Find earliest hit across all statics (narrow-phase TOI).
        let mut best: Option<MoveHit> = None;
        for s in statics {
            if let Some(hit) =
                narrow_phase::cast_capsule_against_static(capsule_iso, &capsule_shape, vel, 1.0, s)
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
                let n = {
                    let n_len_sq = hit.normal.norm_squared();
                    if n_len_sq > 1.0e-12 {
                        hit.normal / n_len_sq.sqrt()
                    } else {
                        na::Vector3::zeros()
                    }
                };

                let leftover = dir * (len - travel);
                let slide = leftover - n * leftover.dot(&n);

                remaining = slide;
                last_hit = Some(hit);

                // If the slide is negligible, we're done.
                if slide.norm_squared() <= MIN_MOVE_SQ {
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

/// Broad-phase accelerated variant of `move_capsule_kinematic`.
///
/// Uses a prebuilt accelerator to prune candidate statics (planes + overlapping AABBs)
/// before running the narrow-phase sweep-and-slide on the filtered set.
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
