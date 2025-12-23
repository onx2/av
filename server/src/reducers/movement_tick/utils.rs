//! Shared movement tick utilities.
//!
//! This module contains:
//! - `movement_step_actor`: the per-actor movement/collision update using Rapier's KCC
//! - `movement_tick_for_kind`: iterates moving actors (player vs non-player) using the composite
//!   btree index on `Actor(should_move, is_player)` and persists updates.
//!
//! Notes:
//! - This is intended for "reasonable collision detection" rather than deterministic physics.
//! - Timesteps are passed in from the caller (player vs non-player ticks may clamp differently).
//! - Movement state is stored directly on `Actor` (the `MovementData` table has been removed).
//! - Movement ticks use the composite btree index on `Actor(should_move, is_player)` for fast iteration.
//!
//! Important:
//! - `TransformData.translation` is mixed precision (`DbVec3i16`): `x/z` are meters (`f32`), `y` is quantized (`i16`, `Y_QUANTIZE_STEP_M`).
//! - Physics/steering operates in meters (`f32`). Only `y` needs decode/encode.
//! - Ground probing must use meters when calling `has_support_within(...)`.
//!
//! Stuck detection:
//! - We treat "stuck" as repeatedly failing to make forward progress in the actor's facing direction.
//! - This catches wall-sliding (lateral motion) while holding a move intent without forcing yaw to turn parallel to the wall.
//! - This matches point-and-click movement where there is no strafe mechanic.

use crate::{
    schema::{actor, secondary_stats, transform_data, Actor, KccSettings, TransformData},
    types::MoveIntent,
    utils::has_support_within,
};
use rapier3d::{control::KinematicCharacterController, na, prelude::*};
use shared::utils::{encode_cell_id, to_planar, yaw_from_u8, yaw_from_xz, yaw_to_u8};
use spacetimedb::ReducerContext;

type Planar = na::Vector2<f32>;

/// if we're not trying to move at least this much, don't evaluate stuck
const STUCK_DESIRED_EPSILON_M: f32 = 0.10;
/// per-tick forward progress threshold (meters)
const STUCK_FORWARD_EPSILON_M: f32 = 0.02;
/// forward progress threshold scales with desired step
const STUCK_MIN_FRACTION: f32 = 0.02;
const STUCK_GRACE_THRESHOLD_STEPS: u8 = 60;

fn apply_facing_progress_stuck_detection(
    actor: &mut Actor,
    actor_dirty: &mut bool,
    prev_translation: &Vector<f32>,
    new_translation: &Vector<f32>,
    desired_planar: &Planar,
    facing_yaw_u8: u8,
) {
    // Facing-progress stuck detection:
    // - We gate on "meaningful desired planar step" (i.e., we were trying to move).
    // - Progress is measured along the actor's facing direction, not along the desired planar step,
    //   so we don't rotate yaw to match wall sliding (looks bad in point-and-click).
    //
    // Micro-opt: gate on squared distance (no sqrt) and only take sqrt when we actually need it.
    let desired_dist_sq = desired_planar.norm_squared();
    let desired_epsilon_sq = STUCK_DESIRED_EPSILON_M * STUCK_DESIRED_EPSILON_M;
    if desired_dist_sq > desired_epsilon_sq && actor.move_intent != MoveIntent::None {
        let desired_dist = desired_dist_sq.sqrt();

        // Facing direction from quantized yaw.
        // Convention: `0..=255` maps uniformly onto `[0, 2π)`.
        let yaw = yaw_from_u8(facing_yaw_u8);
        let facing_dir: Planar = vector![yaw.cos(), yaw.sin()];

        // Build planar delta explicitly to avoid 3D/2D type confusion.
        let actual_planar: Planar = vector![
            new_translation.x - prev_translation.x,
            new_translation.z - prev_translation.z
        ];

        let forward = actual_planar.dot(&facing_dir);

        let forward_threshold = STUCK_FORWARD_EPSILON_M.max(desired_dist * STUCK_MIN_FRACTION);
        let stuck_this_tick = forward < forward_threshold;

        if stuck_this_tick {
            let next = actor.stuck_grace_steps.saturating_add(1);
            if next != actor.stuck_grace_steps {
                actor.stuck_grace_steps = next;
                *actor_dirty = true;
            }
        } else if actor.stuck_grace_steps != 0 {
            actor.stuck_grace_steps = 0;
            *actor_dirty = true;
        }

        if actor.stuck_grace_steps >= STUCK_GRACE_THRESHOLD_STEPS
            && actor.move_intent != MoveIntent::None
        {
            actor.move_intent = MoveIntent::None;
            actor.stuck_grace_steps = 0;
            *actor_dirty = true;
        }
    } else if actor.stuck_grace_steps != 0 {
        // Not attempting meaningful movement this tick -> clear grace so we don't "bank" stuck time.
        actor.stuck_grace_steps = 0;
        *actor_dirty = true;
    }
}

pub fn movement_step_actor(
    ctx: &ReducerContext,
    query_pipeline: &QueryPipeline<'_>,
    controller: &KinematicCharacterController,
    kcc: &KccSettings,
    dt: f32,
    mut actor: Actor,
    mut transform: TransformData,
) -> (Actor, TransformData, bool) {
    let mut actor_dirty = false;
    let capsule_half_height = actor.capsule_half_height;
    let capsule_radius = actor.capsule_radius;

    // Cache previous translation for forward-progress stuck detection.
    let prev_translation: Vector<f32> = transform.translation.into();

    // Determine desired planar movement for this step.
    let current_planar = to_planar(&transform.translation.into());
    let target_planar = match &actor.move_intent {
        MoveIntent::Point(p) => to_planar(&p.into()),
        _ => current_planar,
    };

    let mut desired_planar: Planar = ctx
        .db
        .secondary_stats()
        .id()
        .find(actor.secondary_stats_id)
        .and_then(|row| {
            let max_step = row.movement_speed * dt;
            let displacement = target_planar - current_planar;
            let dist_sq = displacement.norm_squared();
            if dist_sq <= kcc.point_acceptance_radius_sq {
                actor.move_intent = MoveIntent::None;
                actor_dirty = true;
                None
            } else {
                let dist = dist_sq.sqrt();
                let desired_planar = displacement * (max_step.min(dist) / dist);
                if let Some(yaw) = yaw_from_xz(&desired_planar) {
                    transform.yaw = yaw_to_u8(yaw);
                }
                Some(desired_planar)
            }
        })
        .unwrap_or_else(|| vector![0.0, 0.0]);

    let supported = if actor.grounded {
        true
    } else {
        log::warn!("No support autodetected, calculating...");
        has_support_within(
            query_pipeline,
            &transform.translation,
            capsule_half_height,
            capsule_radius,
            0.15,
            kcc.max_slope_climb_deg.to_radians().cos(),
        )
    };

    // Apply down-bias always; apply fall speed only if we were airborne last step.
    let down_bias = -kcc.grounded_down_bias_mps * dt;
    let gravity = if supported {
        0.0
    } else {
        desired_planar *= 0.35;
        -kcc.fall_speed_mps * dt
    };
    let desired_translation = vector![desired_planar[0], down_bias + gravity, desired_planar[1]];

    // KCC move against the static collision world.
    let corrected = controller.move_shape(
        dt,
        query_pipeline,
        &Capsule::new_y(capsule_half_height, capsule_radius),
        &transform.translation.into(),
        desired_translation,
        |_| {},
    );

    // Apply corrected movement
    transform.translation.x += corrected.translation.x;
    transform.translation.y += corrected.translation.y;
    transform.translation.z += corrected.translation.z;

    // Forward-progress stuck detection (extracted helper).
    if supported {
        let new_translation: Vector<f32> = transform.translation.into();
        apply_facing_progress_stuck_detection(
            &mut actor,
            &mut actor_dirty,
            &prev_translation,
            &new_translation,
            &desired_planar,
            transform.yaw,
        );
    }

    // Update cell id on actor when crossing cells (cell encoding expects meters).
    let new_cell_id = encode_cell_id(transform.translation.x, transform.translation.z);
    if new_cell_id != actor.cell_id {
        actor.cell_id = new_cell_id;
        actor_dirty = true;
    }

    // Only update grounded state when it has changed
    if actor.grounded != corrected.grounded {
        actor.grounded = corrected.grounded;
        actor_dirty = true;
    }

    // Actor should move when it has a movement intent or is not grounded.
    let new_should_move = actor.move_intent != MoveIntent::None || !actor.grounded;
    if actor.should_move != new_should_move {
        actor.should_move = new_should_move;
        actor_dirty = true;
    }

    (actor, transform, actor_dirty)
}

/// Shared iteration + update loop for one movement tick "kind" (player vs non-player).
///
/// Iterates only actors that are currently marked `should_move=true` and match `is_player` using the
/// composite btree index on `Actor(should_move, is_player)`, then:
/// - loads `TransformData` row
/// - runs `movement_step_actor`
/// - persists updates
pub fn movement_tick_for_kind(
    ctx: &ReducerContext,
    query_pipeline: &QueryPipeline<'_>,
    controller: &KinematicCharacterController,
    kcc: &KccSettings,
    dt: f32,
    is_player: bool,
) {
    for actor in ctx
        .db
        .actor()
        .should_move_and_is_player()
        .filter((true, is_player))
    {
        let Some(transform) = ctx.db.transform_data().id().find(actor.transform_data_id) else {
            continue;
        };

        let (actor, transform, actor_dirty) =
            movement_step_actor(ctx, query_pipeline, controller, kcc, dt, actor, transform);

        ctx.db.transform_data().id().update(transform);
        if actor_dirty {
            ctx.db.actor().id().update(actor);
        }
    }
}
