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
//! - We keep `Actor.should_move` duplicated and consistent with `MovementData.should_move` because
//!   movement ticks use `Actor(should_move, is_player)` indexing for fast iteration.

use crate::{
    // Import the generated schema modules so `ctx.db.*()` accessors exist in this module.
    schema::{
        actor, movement_data, secondary_stats, transform_data, Actor, KccSettings, MovementData,
        TransformData,
    },
    types::MoveIntent,
    utils::has_support_within,
};
use rapier3d::control::KinematicCharacterController;
use rapier3d::prelude::*;
use shared::utils::{encode_cell_id, yaw_from_xz, UtilMath};
use spacetimedb::ReducerContext;

/// Performs a single movement step for one actor.
///
/// Returns `(actor, movement, transform, actor_dirty)` where:
/// - `actor_dirty` indicates whether the Actor row should be updated (because `cell_id` and/or
///   `should_move` changed).
pub fn movement_step_actor(
    ctx: &ReducerContext,
    query_pipeline: &QueryPipeline<'_>,
    controller: &KinematicCharacterController,
    kcc: &KccSettings,
    dt: f32,
    mut actor: Actor,
    mut movement: MovementData,
    mut transform: TransformData,
) -> (Actor, MovementData, TransformData, bool) {
    let mut actor_dirty = false;

    let capsule_half_height = actor.capsule_half_height;
    let capsule_radius = actor.capsule_radius;

    // Determine desired planar movement for this step.
    let (target_x, target_z, has_point_intent) = match &movement.move_intent {
        MoveIntent::Point(p) => (p.x, p.z, true),
        _ => (transform.translation.x, transform.translation.z, false),
    };

    // Compute intended planar direction toward the target.
    let dx = target_x - transform.translation.x;
    let dz = target_z - transform.translation.z;

    let secondary = ctx
        .db
        .secondary_stats()
        .id()
        .find(actor.secondary_stats_id)
        .unwrap_or_default();

    // Planar step length.
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
        transform.yaw = yaw;
    }

    // Vertical motion.
    // Apply down-bias always; apply fall speed only if we were airborne last step.
    let down_bias = -kcc.grounded_down_bias_mps * dt;
    let gravity = f32::from(!movement.grounded) * (-kcc.fall_speed_mps * dt);

    // KCC move against the static collision world.
    let corrected = controller.move_shape(
        dt,
        query_pipeline,
        &Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
        &Isometry::translation(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ),
        vector![planar.x, down_bias + gravity, planar.z],
        |_| {},
    );

    // Apply corrected movement.
    transform.translation.x += corrected.translation.x;
    transform.translation.y += corrected.translation.y;
    transform.translation.z += corrected.translation.z;

    // Update cell id on actor when crossing cells.
    let new_cell_id = encode_cell_id(transform.translation.x, transform.translation.z);
    if new_cell_id != actor.cell_id {
        actor.cell_id = new_cell_id;
        actor_dirty = true;
    }

    // Persist grounded for the next step.
    if corrected.grounded {
        movement.grounded = true;
        movement.grounded_grace_steps = 8;
    } else if movement.grounded_grace_steps > 0 {
        let supported = has_support_within(
            query_pipeline,
            &transform.translation,
            capsule_half_height,
            capsule_radius,
            kcc.hard_airborne_probe_distance,
            kcc.max_slope_climb_deg.to_radians().cos(),
        );

        if supported {
            movement.grounded_grace_steps -= 1;
        } else {
            movement.grounded_grace_steps = 0;
            movement.grounded = false;
        }
    } else {
        movement.grounded = false;
    }

    // Clear MoveIntent::Point when within acceptance radius (planar).
    if has_point_intent && dist_sq <= kcc.point_acceptance_radius_sq {
        movement.move_intent = MoveIntent::None;
    }

    // Keep `should_move` consistent on both MovementData and Actor (used by movement tick indexes).
    let new_should_move = movement.move_intent != MoveIntent::None || !movement.grounded;
    movement.should_move = new_should_move;
    if actor.should_move != new_should_move {
        actor.should_move = new_should_move;
        actor_dirty = true;
    }

    (actor, movement, transform, actor_dirty)
}

/// Shared iteration + update loop for one movement tick "kind" (player vs non-player).
///
/// Iterates only actors that are currently marked `should_move=true` and match `is_player` using the
/// composite btree index on `Actor(should_move, is_player)`, then:
/// - loads `MovementData` + `TransformData` rows
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
        let Some(movement) = ctx.db.movement_data().id().find(actor.movement_data_id) else {
            continue;
        };

        let Some(transform) = ctx.db.transform_data().id().find(actor.transform_data_id) else {
            continue;
        };

        let (actor, movement, transform, actor_dirty) = movement_step_actor(
            ctx,
            query_pipeline,
            controller,
            kcc,
            dt,
            actor,
            movement,
            transform,
        );

        ctx.db.transform_data().id().update(transform);
        ctx.db.movement_data().id().update(movement);
        if actor_dirty {
            ctx.db.actor().id().update(actor);
        }
    }
}
