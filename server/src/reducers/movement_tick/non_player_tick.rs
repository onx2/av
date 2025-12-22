//! Non-player movement tick.
//!
//! This file contains:
//! - `NonPlayerMovementTickTimer` scheduled table
//! - `init(ctx)` to schedule the tick
//! - `non_player_movement_tick_reducer` scheduled reducer
//!
//! Notes:
//! - Timers are colocated with reducers (as requested).
//! - Uses a clamped variable timestep for "reasonable collision detection" rather than
//!   deterministic physics.
//! - Iterates only moving non-players via the composite btree index on
//!   `Actor(should_move, is_player)` (implemented in `super::utils::movement_tick_for_kind`).
//! - IMPORTANT: `Timestamp` is not filterable for ranged index scans, so we rely on the
//!   `(should_move, is_player)` composite index for iteration.

use crate::{
    reducers::movement_tick::{constants, utils::movement_tick_for_kind},
    // Import generated schema modules so `ctx.db.*()` accessors exist in this module.
    schema::kcc_settings,
    utils::{get_fixed_delta_time, get_variable_delta_time},
    world::{get_kcc, get_rapier_world},
};

use rapier3d::prelude::QueryFilter;
use spacetimedb::{ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};

/// Scheduled timer for the non-player movement tick.
///
/// IMPORTANT:
/// Scheduled tables must include a `scheduled_id: u64` primary key with `#[auto_inc]`.
#[spacetimedb::table(
    name = non_player_movement_tick_timer,
    scheduled(non_player_movement_tick_reducer)
)]
pub struct NonPlayerMovementTickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,

    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: ScheduleAt,

    /// Timestamp of the previous invocation (authoritative delta time source).
    pub last_tick: Timestamp,
}

/// Schedule the non-player movement tick.
pub fn init(ctx: &ReducerContext) {
    let interval = TimeDuration::from_micros(1_000_000i64 / constants::NON_PLAYER_TICK_HZ);

    // Single-row scheduled job.
    ctx.db
        .non_player_movement_tick_timer()
        .scheduled_id()
        .delete(1);

    ctx.db
        .non_player_movement_tick_timer()
        .insert(NonPlayerMovementTickTimer {
            scheduled_id: 1,
            scheduled_at: ScheduleAt::Interval(interval),
            last_tick: ctx.timestamp,
        });
}

#[spacetimedb::reducer]
pub fn non_player_movement_tick_reducer(
    ctx: &ReducerContext,
    mut timer: NonPlayerMovementTickTimer,
) -> Result<(), String> {
    // Only the server (module identity) may invoke scheduled reducers.
    if ctx.sender != ctx.identity() {
        return Err("`non_player_movement_tick_reducer` may not be invoked by clients.".into());
    }

    // Compute real elapsed time since last tick; fallback to scheduled fixed dt.
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);

    // Non-players: looser clamp to tolerate stalls without creating catch-up spirals.
    let dt: f32 = real_dt.clamp(0.0, constants::MAX_NON_PLAYER_DT_S);

    let Some(kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("`non_player_movement_tick_reducer` couldn't find kcc settings.".into());
    };

    let world = get_rapier_world(ctx);
    let controller = get_kcc(ctx);
    let query_pipeline = world.query_pipeline(QueryFilter::default());

    // Process only moving non-players (Actor.should_move=true AND Actor.is_player=false).
    movement_tick_for_kind(ctx, &query_pipeline, &controller, &kcc, dt, false);

    // Persist timer state.
    timer.last_tick = ctx.timestamp;
    ctx.db
        .non_player_movement_tick_timer()
        .scheduled_id()
        .update(timer);

    Ok(())
}
