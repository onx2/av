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

use crate::{
    // Import the generated schema modules so `ctx.db.*()` accessors exist in this module.
    schema::{actor, secondary_stats, transform_data, Actor, KccSettings, TransformData},
    types::MoveIntent,
    utils::has_support_within,
};
use rapier3d::control::KinematicCharacterController;
use rapier3d::prelude::*;
use shared::utils::{encode_cell_id, yaw_from_xz, yaw_to_u8, UtilMath};
use spacetimedb::ReducerContext;

/// Performs a single movement step for one actor.
///
/// Returns `(actor, transform, actor_dirty)` where:
/// - `actor_dirty` is true if the actor row needs updating (cell_id / should_move / movement state changes).
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

    // Determine desired planar movement for this step.
    //
    // NOTE: In this codebase, `TransformData.translation` is already stored in meters as `DbVec3` (f32),
    // so we can use it directly for physics/steering.
    let tx = transform.translation.x;
    let ty = transform.translation.y;
    let tz = transform.translation.z;

    let (target_x, target_z, has_point_intent) = match &actor.move_intent {
        MoveIntent::Point(p) => (p.x, p.z, true),
        _ => (tx, tz, false),
    };

    // Compute intended planar direction toward the target (meters).
    let dx = target_x - tx;
    let dz = target_z - tz;

    let secondary = ctx
        .db
        .secondary_stats()
        .id()
        .find(actor.secondary_stats_id)
        .unwrap_or_default();

    let max_step = if has_point_intent {
        secondary.movement_speed * dt
    } else {
        0.0
    };

    // Avoid sqrt unless we actually need a direction.
    let dist_sq = dx.sq() + dz.sq();
    let planar = if dist_sq > 1.0e-12 && max_step > 0.0 {
        let dist = dist_sq.sqrt();
        let inv_dist = 1.0 / dist;
        let step = max_step.min(dist);
        vector![dx * inv_dist * step, 0.0, dz * inv_dist * step]
    } else {
        vector![0.0, 0.0, 0.0]
    };

    // Update yaw based on intent direction (not post-collision motion).
    if let Some(yaw) = yaw_from_xz(planar.x, planar.z) {
        transform.yaw = yaw_to_u8(yaw);
    }

    let supported = has_support_within(
        query_pipeline,
        &transform.translation,
        capsule_half_height,
        capsule_radius,
        0.525,
        kcc.max_slope_climb_deg.to_radians().cos(),
    );

    // Vertical motion.
    // Apply down-bias always; apply fall speed only if we were airborne last step.
    let down_bias = -kcc.grounded_down_bias_mps * dt;
    let gravity = if supported {
        0.0
    } else {
        -kcc.fall_speed_mps * dt
    };

    // KCC move against the static collision world.
    let corrected = controller.move_shape(
        dt,
        query_pipeline,
        &Capsule::new_y(capsule_half_height, capsule_radius),
        &Isometry::translation(tx, ty, tz),
        vector![planar.x, down_bias + gravity, planar.z],
        |_| {},
    );

    // Apply corrected movement (meters).
    let new_x = tx + corrected.translation.x;
    let new_y = ty + corrected.translation.y;
    let new_z = tz + corrected.translation.z;

    // Persist back into the row type (meters).
    transform.translation.x = new_x;
    transform.translation.y = new_y;
    transform.translation.z = new_z;

    // Update cell id on actor when crossing cells (cell encoding expects meters).
    let new_cell_id = encode_cell_id(new_x, new_z);
    if new_cell_id != actor.cell_id {
        actor.cell_id = new_cell_id;
        actor_dirty = true;
    }

    actor.grounded = corrected.grounded;
    // Clear MoveIntent::Point when within acceptance radius (planar).
    if has_point_intent && dist_sq <= kcc.point_acceptance_radius_sq {
        actor.move_intent = MoveIntent::None;
        actor_dirty = true;
    }

    // Keep `should_move` consistent on Actor (movement ticks iterate `Actor(should_move, is_player)`):
    // - should_move if we have an intent, OR if we're airborne (gravity needs processing).
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
