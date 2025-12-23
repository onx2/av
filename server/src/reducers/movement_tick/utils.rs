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
    schema::{actor, secondary_stats, transform_data, Actor, KccSettings, TransformData},
    types::MoveIntent,
    utils::has_support_within,
};
use rapier3d::control::KinematicCharacterController;
use rapier3d::prelude::*;
use shared::utils::{encode_cell_id, to_planar, yaw_from_xz, yaw_to_u8};
use spacetimedb::ReducerContext;

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
    let current_planar = to_planar(&transform.translation.into());
    let target_planar = match &actor.move_intent {
        MoveIntent::Point(p) => to_planar(&p.into()),
        _ => current_planar,
    };

    let mut desired_planar = ctx
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
            0.75,
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
