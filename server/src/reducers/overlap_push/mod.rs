//! Idle-player overlap push tick.
//!
//! Goal:
//! - Periodically nudge *idle* players (Actor.should_move == false) out of planar (XZ) overlaps.
//! - Avoid O(N^2) by only considering nearby actors in the *same* cell (`cell_id`).
//! - Keep the tick low-frequency (10Hz) and bounded (limits per-actor neighbors + max push step).
//!
//! Notes:
//! - This does NOT attempt full movement simulation. It only computes a planar separation vector
//!   and applies it through Rapier's KCC `move_shape` to respect world collisions.
//! - This is intentionally "players only" for now. If desired later, you can extend to push players
//!   away from non-players too (still only moving the idle player).
//! - IMPORTANT: When two idle players overlap, we only move the *most recently idle* actor.
//!   This prevents a newly-stopped/moving player from displacing an actor that has been standing still.
//!
//! Implementation strategy:
//! 1) Iterate idle players using the composite index `Actor(should_move, is_player)`.
//! 2) For each idle player, gather neighbor actors from the same `cell_id`.
//! 3) If overlapping an idle neighbor that has been idle longer, skip pushing this actor.
//! 4) Otherwise compute the accumulated separation push in XZ for any overlapping capsules.
//! 5) Clamp push magnitude and apply via KCC with Y motion = 0.
//! 6) Persist updated TransformData and Actor.cell_id if it changes.
//!
//! Performance controls:
//! - Only idle players are processed.
//! - Neighbor set is limited by AOI cells and `OVERLAP_PUSH_MAX_NEIGHBORS_PER_ACTOR`.
//! - Per-actor push is clamped by `OVERLAP_PUSH_MAX_STEP_M`.

use crate::{
    // Generated schema modules so `ctx.db.*()` accessors exist.
    schema::{actor, kcc_settings, transform_data},
    utils::{get_fixed_delta_time, get_variable_delta_time},
    world::{get_kcc, get_rapier_world},
};

use rapier3d::prelude::{Isometry, QueryFilter};
use spacetimedb::{ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};

use shared::{
    constants::{
        IDLE_OVERLAP_PUSH_TICK_HZ, OVERLAP_PUSH_MAX_NEIGHBORS_PER_ACTOR, OVERLAP_PUSH_MAX_STEP_M,
        OVERLAP_PUSH_SKIN_M, Y_QUANTIZE_STEP_M,
    },
    utils::{encode_cell_id, UtilMath},
};

use crate::types::MoveIntent;

/// Scheduled timer for the overlap push tick.
///
/// IMPORTANT:
/// Scheduled tables must include a `scheduled_id: u64` primary key with `#[auto_inc]`.
#[spacetimedb::table(name = idle_overlap_push_tick_timer, scheduled(idle_overlap_push_tick_reducer))]
pub struct IdleOverlapPushTickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,

    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: ScheduleAt,

    /// Timestamp of the previous invocation (authoritative delta time source).
    pub last_tick: Timestamp,
}

/// Schedule the idle overlap push tick (10Hz by default).
pub fn init(ctx: &ReducerContext) {
    let interval = TimeDuration::from_micros(1_000_000i64 / IDLE_OVERLAP_PUSH_TICK_HZ);

    // Single-row scheduled job.
    ctx.db
        .idle_overlap_push_tick_timer()
        .scheduled_id()
        .delete(1);
    ctx.db
        .idle_overlap_push_tick_timer()
        .insert(IdleOverlapPushTickTimer {
            scheduled_id: 1,
            scheduled_at: ScheduleAt::Interval(interval),
            last_tick: ctx.timestamp,
        });
}

/// Returns a planar (XZ) separation push vector (meters) to resolve overlaps.
///
/// - `self_pos` and `other_pos` are world meters.
/// - push is accumulated across all neighbors and clamped by caller.
/// - only XZ is affected; Y is ignored.
fn accumulate_planar_separation(
    self_x: f32,
    self_z: f32,
    self_radius: f32,
    other_x: f32,
    other_z: f32,
    other_radius: f32,
) -> (f32, f32) {
    let dx = self_x - other_x;
    let dz = self_z - other_z;

    let dist_sq = dx.sq() + dz.sq();
    // Combined radii with small skin.
    let min_dist = self_radius + other_radius + OVERLAP_PUSH_SKIN_M;
    let min_dist_sq = min_dist.sq();

    if dist_sq >= min_dist_sq {
        return (0.0, 0.0);
    }

    // If positions are extremely close, choose a stable arbitrary direction to avoid NaNs.
    if dist_sq <= 1.0e-12 {
        // Push along +X by the full penetration.
        return (min_dist, 0.0);
    }

    let dist = dist_sq.sqrt();
    let inv_dist = 1.0 / dist;

    // Penetration depth in meters (how far we need to separate centers).
    let penetration = min_dist - dist;

    // Move the idle actor fully (not half) since we're only correcting idle actors.
    let push_x = dx * inv_dist * penetration;
    let push_z = dz * inv_dist * penetration;

    (push_x, push_z)
}

/// Clamp a planar vector to a maximum magnitude, returning the clamped vector.
fn clamp_planar(x: f32, z: f32, max_len: f32) -> (f32, f32) {
    let len_sq = x.sq() + z.sq();
    if len_sq <= max_len.sq() || len_sq <= 1.0e-12 {
        return (x, z);
    }
    let len = len_sq.sqrt();
    let s = max_len / len;
    (x * s, z * s)
}

#[spacetimedb::reducer]
pub fn idle_overlap_push_tick_reducer(
    ctx: &ReducerContext,
    mut timer: IdleOverlapPushTickTimer,
) -> Result<(), String> {
    // Only the server (module identity) may invoke scheduled reducers.
    if ctx.sender != ctx.identity() {
        return Err("`idle_overlap_push_tick_reducer` may not be invoked by clients.".into());
    }

    // Compute elapsed time since last tick (we don't heavily use dt here, but it's useful for future
    // scaling and for consistency with other scheduled ticks).
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);
    let _dt: f32 = real_dt.max(0.0);

    let Some(_kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("`idle_overlap_push_tick_reducer` couldn't find kcc settings.".into());
    };

    // Setup Rapier query pipeline and controller.
    let world = get_rapier_world(ctx);
    let controller = get_kcc(ctx);
    let query_pipeline = world.query_pipeline(QueryFilter::default());

    // Iterate idle players only (should_move=false, is_player=true).
    for actor in ctx
        .db
        .actor()
        .should_move_and_is_player()
        .filter((false, true))
    {
        let Some(mut transform) = ctx.db.transform_data().id().find(actor.transform_data_id) else {
            continue;
        };

        // Only apply overlap pushing to actors that are explicitly Idle (movement state machine).
        let self_idle_since_us = match actor.move_intent {
            MoveIntent::Idle(since_us) => since_us,
            _ => continue,
        };

        // Decode mixed-precision translation (x/z are meters, y is quantized).
        let tx = transform.translation.x;
        let ty = transform.translation.y as f32 * Y_QUANTIZE_STEP_M;
        let tz = transform.translation.z;

        // Gather candidate neighbors from the actor's current cell only.
        // This is a deliberate perf tradeoff: we may miss rare cross-cell overlaps near boundaries.
        let cell_id = actor.cell_id;

        let mut neighbors_checked: usize = 0;
        let mut push_x_acc: f32 = 0.0;
        let mut push_z_acc: f32 = 0.0;

        // If we overlap an idle neighbor that has been idle longer than us, we should NOT move.
        // This prevents a newly-idled player from displacing an older idle player.
        let mut is_anchored_by_older_idle: bool = false;

        // We only push the idle actor. Neighbors can be any actor, but we will only consider
        // actors that have a valid transform.
        //
        // This assumes `cell_id` is indexed and provides a filter iterator.
        for other in ctx.db.actor().cell_id().filter(cell_id) {
            if neighbors_checked >= OVERLAP_PUSH_MAX_NEIGHBORS_PER_ACTOR {
                break;
            }

            // Skip self.
            if other.id == actor.id {
                continue;
            }

            // Optional: ignore non-players entirely for now (per your preference).
            if !other.is_player {
                continue;
            }

            // Optional: ignore actors that are moving; we only correct idle stacking.
            if other.should_move {
                continue;
            }

            // Only compare against other idle actors that have an idle timestamp.
            let other_idle_since_us = match other.move_intent {
                MoveIntent::Idle(since_us) => since_us,
                _ => continue,
            };

            let Some(other_t) = ctx.db.transform_data().id().find(other.transform_data_id) else {
                continue;
            };

            let ox = other_t.translation.x;
            let oz = other_t.translation.z;

            let (px, pz) = accumulate_planar_separation(
                tx,
                tz,
                actor.capsule_radius,
                ox,
                oz,
                other.capsule_radius,
            );

            // If there's no overlap, skip.
            if px == 0.0 && pz == 0.0 {
                neighbors_checked += 1;
                continue;
            }

            // Anchoring rule:
            // - Smaller `since_us` means "idle for longer" (older idle) => that actor should stay anchored.
            // - Therefore, if *we* are older-idle than the neighbor, we should NOT move.
            //
            // This ensures a newly-spawned/newly-idled actor yields to an actor that has been idle longer.
            if self_idle_since_us < other_idle_since_us {
                is_anchored_by_older_idle = true;
                break;
            }

            // Otherwise, we are at least as old-idle as the neighbor; allow pushing ourselves.
            push_x_acc += px;
            push_z_acc += pz;

            neighbors_checked += 1;
        }

        // If we're anchored by an older idle neighbor, do not move this actor.
        if is_anchored_by_older_idle {
            continue;
        }

        // Nothing to do.
        if push_x_acc.sq() + push_z_acc.sq() <= 1.0e-12 {
            continue;
        }

        // Clamp push so we don't teleport in pathological overlaps.
        let (push_x, push_z) = clamp_planar(push_x_acc, push_z_acc, OVERLAP_PUSH_MAX_STEP_M);

        // Apply through KCC so we don't push into walls. No vertical movement applied here.
        let corrected = controller.move_shape(
            // This is a purely corrective pass; we can treat dt as 0 for kinematic resolution,
            // but Rapier's API takes `dt`. Use a small positive value to avoid surprising behavior.
            // (We use `fixed_dt`, which corresponds to the schedule interval.)
            fixed_dt.max(1.0e-6),
            &query_pipeline,
            &rapier3d::prelude::Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
            &Isometry::translation(tx, ty, tz),
            rapier3d::prelude::vector![push_x, 0.0, push_z],
            |_| {},
        );

        // Persist corrected translation.
        let new_x = tx + corrected.translation.x;
        let new_y = ty + corrected.translation.y; // should be ~0, but keep generic
        let new_z = tz + corrected.translation.z;

        transform.translation.x = new_x;
        transform.translation.z = new_z;
        transform.translation.y = (new_y / Y_QUANTIZE_STEP_M)
            .round()
            .clamp(i16::MIN as f32, i16::MAX as f32) as i16;

        ctx.db.transform_data().id().update(transform);

        // Update actor cell_id if it changed.
        let new_cell_id = encode_cell_id(new_x, new_z);
        if new_cell_id != actor.cell_id {
            let mut actor_updated = actor;
            actor_updated.cell_id = new_cell_id;
            ctx.db.actor().id().update(actor_updated);
        }
    }

    // Persist timer state.
    timer.last_tick = ctx.timestamp;
    ctx.db
        .idle_overlap_push_tick_timer()
        .scheduled_id()
        .update(timer);

    Ok(())
}
