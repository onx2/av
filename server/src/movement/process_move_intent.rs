use crate::{
    movement_state_tbl, row_to_def, world_static_tbl, MoveIntentData, SecondaryStatsRow,
    TransformRow, Vec2,
};
use nalgebra::{UnitQuaternion, Vector2, Vector3};
use rapier3d::{
    control::{CharacterAutostep, KinematicCharacterController},
    parry::utils::hashmap::HashMap,
    prelude::{Capsule, QueryFilter},
};
use shared::{
    constants::{GRAVITY, TERMINAL_VELOCITY},
    encode_cell_id, get_desired_delta, is_at_target_planar,
    utils::build_static_query_world,
    yaw_from_xz, Owner,
};
use spacetimedb::{reducer, ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};

pub fn delta_time(now: Timestamp, last: Timestamp) -> Option<f32> {
    now.time_duration_since(last)
        .map(|dur| dur.to_micros() as f32 / 1_000_000.0)
}

#[spacetimedb::table(
    name = movement_tick_timer,
    scheduled(process_move_intent_reducer)
)]
pub struct ProcessMoveIntentTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,

    // Custom data for scheduled reducer:
    pub last_tick: Timestamp,
}

pub fn init_process_move_intent(ctx: &ReducerContext) {
    ctx.db.movement_tick_timer().iter().for_each(|row| {
        ctx.db.movement_tick_timer().delete(row);
    });
    ctx.db.movement_tick_timer().insert(ProcessMoveIntentTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Time(ctx.timestamp),
        last_tick: ctx.timestamp,
    });
}

fn reschedule(ctx: &ReducerContext, timer: ProcessMoveIntentTimer) {
    ctx.db
        .movement_tick_timer()
        .scheduled_id()
        .delete(timer.scheduled_id);
    ctx.db.movement_tick_timer().insert(ProcessMoveIntentTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Time(ctx.timestamp + TimeDuration::from_micros(100_000)),
        last_tick: ctx.timestamp,
    });
}

#[reducer]
fn process_move_intent_reducer(
    ctx: &ReducerContext,
    timer: ProcessMoveIntentTimer,
) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("`movement_tick_reducer` may not be invoked by clients.".into());
    }
    let Some(dt) = delta_time(ctx.timestamp, timer.last_tick).map(|dt| dt.min(0.08)) else {
        return Err("Failed to calculate delta time".into());
    };

    let kcc = KinematicCharacterController {
        autostep: Some(CharacterAutostep {
            include_dynamic_bodies: false,
            ..CharacterAutostep::default()
        }),
        ..KinematicCharacterController::default()
    };

    // Build the rapier physics world
    let world_defs = ctx.db.world_static_tbl().iter().map(row_to_def);
    let query_world = build_static_query_world(world_defs, dt);
    let query_pipeline = query_world.as_query_pipeline(QueryFilter::only_fixed());

    // Initialize a actor location cache. Rapier exposes a much faster HashMap, 10x fewer CPU instructions.
    // We no longer have a move_intent table; size this off of movement_state rows.
    let moving_count = ctx.db.movement_state_tbl().count() as usize;
    let mut target_xz_cache: HashMap<Owner, Vec2> =
        HashMap::with_capacity_and_hasher(moving_count, Default::default());
    let view_ctx = ctx.as_read_only();

    for mut movement_state in ctx.db.movement_state_tbl().should_move().filter(true) {
        let owner = movement_state.owner;
        let Some(mut owner_transform) = TransformRow::find(ctx, owner) else {
            continue;
        };
        let current_xz: Vector2<f32> = owner_transform.data.translation.xz().into();
        let target_xz: Vector2<f32> = movement_state
            .move_intent
            .as_ref()
            .map(|mi| {
                mi.target_position_with_cache(&view_ctx.db, &mut target_xz_cache)
                    .map(|pos| pos.into())
                    .unwrap_or(current_xz)
            })
            .unwrap_or(current_xz);

        if !movement_state.grounded && movement_state.vertical_velocity < TERMINAL_VELOCITY {
            movement_state.vertical_velocity += GRAVITY * dt;
        }

        let Some(speed) = SecondaryStatsRow::find(&view_ctx, owner)
            .map(|secondary_stats| secondary_stats.data.movement_speed)
        else {
            log::error!("Failed to find secondary stats for entity {}", owner);
            continue;
        };

        let direction = (target_xz - current_xz)
            .try_normalize(0.0)
            .unwrap_or_default();

        if let Some(yaw) = yaw_from_xz(direction) {
            owner_transform.data.rotation =
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw).into();
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
                current_xz,
                target_xz,
                speed,
                movement_state.vertical_velocity,
                movement_state.grounded,
                dt,
            ),
            |_| {},
        );
        owner_transform.data.translation.x += correction.translation.x;
        owner_transform.data.translation.y += correction.translation.y;
        owner_transform.data.translation.z += correction.translation.z;

        movement_state.grounded = correction.grounded;
        if movement_state.grounded {
            movement_state.vertical_velocity = 0.0;
        }
        movement_state.cell_id = encode_cell_id(
            owner_transform.data.translation.x,
            owner_transform.data.translation.z,
        );

        if is_at_target_planar(owner_transform.data.translation.xz().into(), target_xz) {
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
            }
        }
        movement_state.should_move = movement_state.move_intent.is_some() || !correction.grounded;

        owner_transform.update(ctx, owner_transform.data);
        ctx.db.movement_state_tbl().owner().update(movement_state);
    }

    // Delete the old scheduled reducer and create a new one instead up updating an "ScheduledAt::Interval"
    // version because of an engine bug that considers time elapsed in reducer as part of the Interval.
    // https://discord.com/channels/1037340874172014652/1455222615747858534
    reschedule(ctx, timer);

    Ok(())
}
