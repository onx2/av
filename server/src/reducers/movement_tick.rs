use crate::{
    schema::{actor, kcc_settings, movement_data, secondary_stats, transform_data},
    types::MoveIntent,
    utils::{get_fixed_delta_time, get_variable_delta_time, has_support_within, LogStopwatch},
    world::{get_kcc, get_rapier_world},
};
use rapier3d::prelude::*;
use shared::utils::{encode_cell_id, yaw_from_xz, UtilMath};
use spacetimedb::*;

/// Safety cap to avoid spending unbounded time catching up after long stalls.
const MAX_STEPS_PER_TICK: u32 = 3;

/// Player tick rate: 30 Hz.
const PLAYER_TICK_HZ: i64 = 30;

/// Non-player tick rate: 15 Hz.
const NON_PLAYER_TICK_HZ: i64 = 10;

/// Scheduled timer for the player movement tick.
#[table(name = player_movement_tick_timer, scheduled(player_movement_tick_reducer))]
pub struct PlayerMovementTickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: spacetimedb::ScheduleAt,
    /// Timestamp of the previous invocation (authoritative delta time source).
    pub last_tick: Timestamp,
    /// The time deficit left over from the last run.
    pub time_accumulator: f32,
}

/// Scheduled timer for the non-player movement tick.
#[table(name = non_player_movement_tick_timer, scheduled(non_player_movement_tick_reducer))]
pub struct NonPlayerMovementTickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: spacetimedb::ScheduleAt,
    /// Timestamp of the previous invocation (authoritative delta time source).
    pub last_tick: Timestamp,
}

/// Initialize both movement tick schedules.
pub fn init(ctx: &ReducerContext) {
    // Player movement tick @ 30 Hz.
    let player_interval = TimeDuration::from_micros(1_000_000i64 / PLAYER_TICK_HZ);
    ctx.db.player_movement_tick_timer().scheduled_id().delete(1);
    ctx.db
        .player_movement_tick_timer()
        .insert(PlayerMovementTickTimer {
            scheduled_id: 1,
            scheduled_at: spacetimedb::ScheduleAt::Interval(player_interval),
            last_tick: ctx.timestamp,
            time_accumulator: 0.0,
        });

    // Non-player movement tick @ 15 Hz.
    let npc_interval = TimeDuration::from_micros(1_000_000i64 / NON_PLAYER_TICK_HZ);
    ctx.db
        .non_player_movement_tick_timer()
        .scheduled_id()
        .delete(1);
    ctx.db
        .non_player_movement_tick_timer()
        .insert(NonPlayerMovementTickTimer {
            scheduled_id: 1,
            scheduled_at: spacetimedb::ScheduleAt::Interval(npc_interval),
            last_tick: ctx.timestamp,
        });
}

#[spacetimedb::reducer]
pub fn player_movement_tick_reducer(
    ctx: &ReducerContext,
    mut timer: PlayerMovementTickTimer,
) -> Result<(), String> {
    // Sampled span logging (LogStopwatch-style; WASM-safe).
    let mut sw = LogStopwatch::new(ctx, "player_movement_tick", false, 0.05);

    sw.span("auth");
    if ctx.sender != ctx.identity() {
        return Err("`player_movement_tick_reducer` may not be invoked by clients.".into());
    }

    sw.span("dt_calc");
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);

    sw.span("accumulate");
    timer.time_accumulator += real_dt;

    sw.span("load_settings");
    let Some(kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("`player_movement_tick_reducer` couldn't find kcc settings.".into());
    };

    sw.span("world_cache");
    let world = get_rapier_world(ctx);
    let controller = get_kcc(ctx);

    // Build the query pipeline once per reducer invocation (not per fixed-step).
    sw.span("query_pipeline");
    let query_pipeline = world.query_pipeline(QueryFilter::default());

    sw.span("fixed_steps");
    let mut steps_ran: u32 = 0;
    while timer.time_accumulator >= fixed_dt {
        sw.span("actors_loop");

        // Fast path: iterate only moving players using the composite btree index
        // `actor(should_move, is_player)`.
        for mut a in ctx
            .db
            .actor()
            .should_move_and_is_player()
            .filter((true, true))
        {
            sw.span("actor_step");

            let Some(mut m) = ctx.db.movement_data().id().find(a.movement_data_id) else {
                continue;
            };

            let Some(mut t) = ctx.db.transform_data().id().find(a.transform_data_id) else {
                continue;
            };

            // We'll update the actor row only if something actually changes (cell_id / should_move).
            let mut actor_dirty = false;

            let capsule_half_height = a.capsule_half_height;
            let capsule_radius = a.capsule_radius;

            let (target_x, target_z, has_point_intent) = match &m.move_intent {
                MoveIntent::Point(p) => (p.x, p.z, true),
                _ => (t.translation.x, t.translation.z, false),
            };

            let dx = target_x - t.translation.x;
            let dz = target_z - t.translation.z;

            let secondary = ctx
                .db
                .secondary_stats()
                .id()
                .find(a.secondary_stats_id)
                .unwrap_or_default();

            let max_step = if has_point_intent {
                secondary.movement_speed * fixed_dt
            } else {
                0.0
            };

            let dist_sq = dx.sq() + dz.sq();
            let planar = if dist_sq > 1.0e-12 && max_step > 0.0 {
                let dist = dist_sq.sqrt();
                let inv_dist = 1.0 / dist;
                let step = max_step.min(dist);
                vector![dx * inv_dist * step, 0.0, dz * inv_dist * step]
            } else {
                vector![0.0, 0.0, 0.0]
            };

            if let Some(yaw) = yaw_from_xz(planar.x, planar.z) {
                t.yaw = yaw;
            }

            let down_bias = -kcc.grounded_down_bias_mps * fixed_dt;
            let gravity = f32::from(!m.grounded) * (-kcc.fall_speed_mps * fixed_dt);

            sw.span("kcc_move_shape");
            let corrected = controller.move_shape(
                fixed_dt,
                &query_pipeline,
                &Capsule::new_y(a.capsule_half_height, a.capsule_radius),
                &Isometry::translation(t.translation.x, t.translation.y, t.translation.z),
                vector![planar.x, down_bias + gravity, planar.z],
                |_| {},
            );

            t.translation.x += corrected.translation.x;
            t.translation.y += corrected.translation.y;
            t.translation.z += corrected.translation.z;

            let new_cell_id = encode_cell_id(t.translation.x, t.translation.z);
            if new_cell_id != a.cell_id {
                a.cell_id = new_cell_id;
                actor_dirty = true;
            }

            if corrected.grounded {
                m.grounded = true;
                m.grounded_grace_steps = 8;
            } else if m.grounded_grace_steps > 0 {
                sw.span("ground_probe");
                let supported = has_support_within(
                    &query_pipeline,
                    &t.translation,
                    capsule_half_height,
                    capsule_radius,
                    kcc.hard_airborne_probe_distance,
                    kcc.max_slope_climb_deg.to_radians().cos(),
                );

                if supported {
                    m.grounded_grace_steps -= 1;
                } else {
                    m.grounded_grace_steps = 0;
                    m.grounded = false;
                }
            } else {
                m.grounded = false;
            }

            if has_point_intent && dist_sq <= kcc.point_acceptance_radius_sq {
                m.move_intent = MoveIntent::None;
            }

            let new_should_move = m.move_intent != MoveIntent::None || !m.grounded;
            m.should_move = new_should_move;
            if a.should_move != new_should_move {
                a.should_move = new_should_move;
                actor_dirty = true;
            }

            sw.span("db_updates");
            ctx.db.transform_data().id().update(t);
            ctx.db.movement_data().id().update(m);
            if actor_dirty {
                ctx.db.actor().id().update(a);
            }
        }

        sw.span("step_accounting");
        timer.time_accumulator -= fixed_dt;
        steps_ran += 1;

        if steps_ran >= MAX_STEPS_PER_TICK {
            timer.time_accumulator = 0.0;
            break;
        }
    }

    sw.span("persist_timer");
    timer.last_tick = ctx.timestamp;
    ctx.db
        .player_movement_tick_timer()
        .scheduled_id()
        .update(timer);

    sw.end_span();
    Ok(())
}

#[spacetimedb::reducer]
pub fn non_player_movement_tick_reducer(
    ctx: &ReducerContext,
    mut timer: NonPlayerMovementTickTimer,
) -> Result<(), String> {
    // Sampled span logging (LogStopwatch-style; WASM-safe).
    let mut sw = LogStopwatch::new(ctx, "non_player_movement_tick", false, 0.02);

    sw.span("auth");
    if ctx.sender != ctx.identity() {
        return Err("`non_player_movement_tick_reducer` may not be invoked by clients.".into());
    }

    sw.span("dt_calc");
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);

    // Non-player tick uses a variable timestep to avoid accumulator "catch-up spirals".
    // We clamp only extreme stalls to keep collision behavior reasonable.
    const MAX_NPC_DT_S: f32 = 0.25;
    let dt: f32 = real_dt.clamp(0.0, MAX_NPC_DT_S);

    sw.span("load_settings");
    let Some(kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("`non_player_movement_tick_reducer` couldn't find kcc settings.".into());
    };

    sw.span("world_cache");
    let world = get_rapier_world(ctx);
    let controller = get_kcc(ctx);

    // Build the query pipeline once per reducer invocation (not per fixed-step).
    sw.span("query_pipeline");
    let query_pipeline = world.query_pipeline(QueryFilter::default());

    // Single-step processing (no accumulator catch-up loop).
    sw.span("actors_loop");

    // Fast path: iterate only moving non-players using the composite btree index
    // `actor(should_move, is_player)`.
    for mut a in ctx
        .db
        .actor()
        .should_move_and_is_player()
        .filter((true, false))
    {
        sw.span("actor_step");

        let Some(mut m) = ctx.db.movement_data().id().find(a.movement_data_id) else {
            continue;
        };

        let Some(mut t) = ctx.db.transform_data().id().find(a.transform_data_id) else {
            continue;
        };

        // We'll update the actor row only if something actually changes (cell_id / should_move).
        let mut actor_dirty = false;

        let capsule_half_height = a.capsule_half_height;
        let capsule_radius = a.capsule_radius;

        let (target_x, target_z, has_point_intent) = match &m.move_intent {
            MoveIntent::Point(p) => (p.x, p.z, true),
            _ => (t.translation.x, t.translation.z, false),
        };

        let dx = target_x - t.translation.x;
        let dz = target_z - t.translation.z;

        let secondary = ctx
            .db
            .secondary_stats()
            .id()
            .find(a.secondary_stats_id)
            .unwrap_or_default();

        // Planar step length uses the variable dt.
        let max_step = if has_point_intent {
            secondary.movement_speed * dt
        } else {
            0.0
        };

        let dist_sq = dx.sq() + dz.sq();
        let planar = if dist_sq > 1.0e-12 && max_step > 0.0 {
            let dist = dist_sq.sqrt();
            let inv_dist = 1.0 / dist;
            let step = max_step.min(dist);
            vector![dx * inv_dist * step, 0.0, dz * inv_dist * step]
        } else {
            vector![0.0, 0.0, 0.0]
        };

        if let Some(yaw) = yaw_from_xz(planar.x, planar.z) {
            t.yaw = yaw;
        }

        // Vertical components use the variable dt.
        let down_bias = -kcc.grounded_down_bias_mps * dt;
        let gravity = f32::from(!m.grounded) * (-kcc.fall_speed_mps * dt);

        sw.span("kcc_move_shape");
        let corrected = controller.move_shape(
            dt,
            &query_pipeline,
            &Capsule::new_y(a.capsule_half_height, a.capsule_radius),
            &Isometry::translation(t.translation.x, t.translation.y, t.translation.z),
            vector![planar.x, down_bias + gravity, planar.z],
            |_| {},
        );

        t.translation.x += corrected.translation.x;
        t.translation.y += corrected.translation.y;
        t.translation.z += corrected.translation.z;

        let new_cell_id = encode_cell_id(t.translation.x, t.translation.z);
        if new_cell_id != a.cell_id {
            a.cell_id = new_cell_id;
            actor_dirty = true;
        }

        if corrected.grounded {
            m.grounded = true;
            m.grounded_grace_steps = 8;
        } else if m.grounded_grace_steps > 0 {
            sw.span("ground_probe");
            let supported = has_support_within(
                &query_pipeline,
                &t.translation,
                capsule_half_height,
                capsule_radius,
                kcc.hard_airborne_probe_distance,
                kcc.max_slope_climb_deg.to_radians().cos(),
            );

            if supported {
                m.grounded_grace_steps -= 1;
            } else {
                m.grounded_grace_steps = 0;
                m.grounded = false;
            }
        } else {
            m.grounded = false;
        }

        if has_point_intent && dist_sq <= kcc.point_acceptance_radius_sq {
            m.move_intent = MoveIntent::None;
        }

        let new_should_move = m.move_intent != MoveIntent::None || !m.grounded;
        m.should_move = new_should_move;
        if a.should_move != new_should_move {
            a.should_move = new_should_move;
            actor_dirty = true;
        }

        sw.span("db_updates");
        ctx.db.transform_data().id().update(t);
        ctx.db.movement_data().id().update(m);
        if actor_dirty {
            ctx.db.actor().id().update(a);
        }
    }

    sw.span("persist_timer");
    timer.last_tick = ctx.timestamp;
    ctx.db
        .non_player_movement_tick_timer()
        .scheduled_id()
        .update(timer);

    sw.end_span();
    Ok(())
}
