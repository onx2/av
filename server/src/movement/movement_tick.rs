use std::iter::once;

use crate::{
    movement_state_tbl, row_to_def, world_static_tbl, MoveIntentData, SecondaryStatsRow,
    TransformRow, Vec2,
};
use nalgebra::Vector2;
use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    parry::utils::hashmap::HashMap,
    prelude::{Capsule, QueryFilter},
};
use shared::{
    constants::{GRAVITY, TERMINAL_VELOCITY},
    encode_cell_id, get_desired_delta, is_at_target_planar,
    utils::{build_static_query_world, yaw_to_u8},
    yaw_from_xz, Owner,
};
use spacetimedb::{reducer, ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};

pub fn delta_time(now: Timestamp, last: Timestamp) -> Option<f32> {
    now.time_duration_since(last)
        .map(|dur| dur.to_micros() as f32 / 1_000_000.0)
}

#[spacetimedb::table(
    name = movement_tick_timer,
    scheduled(movement_tick_reducer)
)]
pub struct MovementTickTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,

    // Custom data for scheduled reducer:
    pub last_tick: Timestamp,
}

const TICK_INTERVAL_MICROS: i64 = 200_000;

pub fn init_movement_tick(ctx: &ReducerContext) {
    ctx.db.movement_tick_timer().scheduled_id().delete(1);
    ctx.db.movement_tick_timer().insert(MovementTickTimer {
        scheduled_id: 1,
        scheduled_at: ScheduleAt::Interval(TimeDuration::from_micros(TICK_INTERVAL_MICROS)),
        last_tick: ctx.timestamp,
    });
    log::info!("init movement_tick");
}

#[reducer]
fn movement_tick_reducer(ctx: &ReducerContext, mut timer: MovementTickTimer) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        log::error!("`movement_tick_reducer` may not be invoked by clients.");
        return Err("`movement_tick_reducer` may not be invoked by clients.".into());
    }

    // No need to waste CPU instructions and a table scan for building the query world
    // when we don't have processable movement states...
    let mut movement_states = ctx.db.movement_state_tbl().should_move().filter(true);
    let Some(first_movement_state) = movement_states.next() else {
        log::info!("No movement states to process");
        return Ok(());
    };

    let dt = delta_time(ctx.timestamp, timer.last_tick)
        .map(|dt| dt.min(0.125))
        .unwrap_or(0.125);

    let kcc = KinematicCharacterController {
        autostep: Some(CharacterAutostep {
            include_dynamic_bodies: false,
            max_height: CharacterLength::Relative(0.4),
            ..CharacterAutostep::default()
        }),
        offset: CharacterLength::Relative(0.025),
        ..KinematicCharacterController::default()
    };

    // Build the rapier physics world
    let world_defs = ctx.db.world_static_tbl().iter().map(row_to_def);
    let query_world = build_static_query_world(world_defs, dt);
    let query_pipeline = query_world.as_query_pipeline(QueryFilter::only_fixed());

    // Initialize a actor location cache. Rapier exposes a much faster HashMap, 10x fewer CPU instructions.
    let mut target_xz_cache: HashMap<Owner, Vec2> = HashMap::default();
    let view_ctx = ctx.as_read_only();
    for mut movement_state in once(first_movement_state).chain(movement_states) {
        let owner = movement_state.owner;
        let Some(mut owner_transform) = TransformRow::find(ctx, owner) else {
            log::error!("Failed to find transform for owner {}", owner);
            continue;
        };

        let current_planar: Vector2<f32> = owner_transform.data.translation.xz().into();
        let target_planar: Vector2<f32> = movement_state
            .move_intent
            .as_ref()
            .map(|mi| {
                mi.target_position_with_cache(&view_ctx.db, &mut target_xz_cache)
                    .map(|pos| pos.into())
                    .unwrap_or(current_planar)
            })
            .unwrap_or(current_planar);

        let mut movement_state_dirty = false;
        if !movement_state.grounded && movement_state.vertical_velocity > TERMINAL_VELOCITY {
            movement_state.vertical_velocity += GRAVITY * dt;
            movement_state_dirty = true;
        }

        let Some(movement_speed_mps) = SecondaryStatsRow::find(&view_ctx, owner)
            .map(|secondary_stats| secondary_stats.data.movement_speed)
        else {
            log::error!("Failed to find secondary stats for entity {}", owner);
            continue;
        };

        let direction = (target_planar - current_planar)
            .try_normalize(0.0)
            .unwrap_or_default();

        if let Some(yaw) = yaw_from_xz(direction) {
            owner_transform.data.yaw = yaw_to_u8(yaw);
        }

        let correction = kcc.move_shape(
            dt,
            &query_pipeline,
            &Capsule::new_y(
                movement_state.capsule.half_height,
                movement_state.capsule.radius,
            ),
            &owner_transform.data.into(),
            get_desired_delta(
                current_planar,
                target_planar,
                movement_speed_mps,
                movement_state.vertical_velocity,
                movement_state.grounded,
                dt,
            ),
            |_| {},
        );

        owner_transform.data.translation.x += correction.translation.x;
        owner_transform.data.translation.y += correction.translation.y;
        owner_transform.data.translation.z += correction.translation.z;

        if movement_state.grounded != correction.grounded {
            movement_state.grounded = correction.grounded;
            movement_state_dirty = true;
        }
        if movement_state.grounded && movement_state.vertical_velocity != 0.0 {
            movement_state.vertical_velocity = 0.0;
            movement_state_dirty = true;
        }

        let cell_id = encode_cell_id(
            owner_transform.data.translation.x,
            owner_transform.data.translation.z,
        );
        if movement_state.cell_id != cell_id {
            movement_state.cell_id = cell_id;
            movement_state_dirty = true;
        }

        if is_at_target_planar(owner_transform.data.translation.xz().into(), target_planar) {
            let clear_intent = match movement_state.move_intent.as_mut() {
                Some(MoveIntentData::Point(_)) => true,
                Some(MoveIntentData::Actor(_)) => true,
                Some(MoveIntentData::Path(path)) => {
                    if !path.is_empty() {
                        path.remove(0);
                    }
                    path.is_empty()
                }
                None => false,
            };
            if clear_intent {
                movement_state.move_intent = None;
                movement_state_dirty = true;
            }
        }
        let should_move = movement_state.move_intent.is_some() || !correction.grounded;
        if movement_state.should_move != should_move {
            movement_state.should_move = should_move;
            movement_state_dirty = true;
        }

        owner_transform.update_from_self(ctx);
        if movement_state_dirty {
            movement_state.update_from_self(ctx);
        }
    }

    timer.last_tick = ctx.timestamp;
    ctx.db.movement_tick_timer().scheduled_id().update(timer);

    Ok(())
}
