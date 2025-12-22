//! Player movement tick.
//!
//! This file contains:
//! - `PlayerMovementTickTimer` scheduled table
//! - `init(ctx)` to schedule the tick
//! - `player_movement_tick_reducer` scheduled reducer
//!
//! Notes:
//! - Timers are colocated with reducers (as requested).
//! - This uses a clamped variable timestep for "reasonable collision detection" rather than
//!   deterministic physics.
//! - The per-actor iteration/update logic lives in `super::utils`.

use crate::{
    reducers::movement_tick::{constants, utils::movement_tick_for_kind},
    // Import generated schema modules so `ctx.db.*()` accessors exist in this module.
    schema::kcc_settings,
    utils::{get_fixed_delta_time, get_variable_delta_time},
    world::{get_kcc, get_rapier_world},
};

use rapier3d::prelude::QueryFilter;
use spacetimedb::{ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};

/// Scheduled timer for the player movement tick.
///
/// IMPORTANT:
/// Scheduled tables must include a `scheduled_id: u64` primary key with `#[auto_inc]`.
#[spacetimedb::table(name = player_movement_tick_timer, scheduled(player_movement_tick_reducer))]
pub struct PlayerMovementTickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,

    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: ScheduleAt,

    /// Timestamp of the previous invocation (authoritative delta time source).
    pub last_tick: Timestamp,
}

/// Schedule the player movement tick.
pub fn init(ctx: &ReducerContext) {
    let interval = TimeDuration::from_micros(1_000_000i64 / constants::PLAYER_TICK_HZ);

    // Single-row scheduled job.
    ctx.db.player_movement_tick_timer().scheduled_id().delete(1);
    ctx.db
        .player_movement_tick_timer()
        .insert(PlayerMovementTickTimer {
            scheduled_id: 1,
            scheduled_at: ScheduleAt::Interval(interval),
            last_tick: ctx.timestamp,
        });
}

#[spacetimedb::reducer]
pub fn player_movement_tick_reducer(
    ctx: &ReducerContext,
    mut timer: PlayerMovementTickTimer,
) -> Result<(), String> {
    // Only the server (module identity) may invoke scheduled reducers.
    if ctx.sender != ctx.identity() {
        return Err("`player_movement_tick_reducer` may not be invoked by clients.".into());
    }

    // Compute real elapsed time since last tick; fallback to scheduled fixed dt.
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);

    // Players: tighter clamp for responsiveness and to avoid large jumps after stalls.
    let dt: f32 = real_dt.clamp(0.0, constants::MAX_PLAYER_DT_S);

    let Some(kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("`player_movement_tick_reducer` couldn't find kcc settings.".into());
    };

    let world = get_rapier_world(ctx);
    let controller = get_kcc(ctx);
    let query_pipeline = world.query_pipeline(QueryFilter::default());

    // Process only moving players (Actor.should_move=true AND Actor.is_player=true).
    movement_tick_for_kind(ctx, &query_pipeline, &controller, &kcc, dt, true);

    // Persist timer state.
    timer.last_tick = ctx.timestamp;
    ctx.db
        .player_movement_tick_timer()
        .scheduled_id()
        .update(timer);

    Ok(())
}
