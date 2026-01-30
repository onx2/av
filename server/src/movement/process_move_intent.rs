use crate::{
    move_intent_tbl, movement_state_tbl, MoveIntentData, MovementState, SecondaryStats, Transform,
    Vec2,
};
use nalgebra::{UnitQuaternion, Vector, Vector2, Vector3};
use rapier3d::{
    control::{CharacterAutostep, KinematicCharacterController},
    prelude::{Capsule, QueryFilter},
};
use shared::{
    constants::{GRAVITY, TERMINAL_VELOCITY},
    encode_cell_id, get_desired_delta, is_at_target_planar,
    utils::build_static_query_world,
    yaw_from_xz, ColliderShapeDef, Owner, WorldStaticDef,
};
use spacetimedb::{reducer, ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};
use std::collections::HashMap;

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
    // TODO: Actually build world
    let query_world = build_static_query_world(
        [WorldStaticDef {
            id: 1,
            translation: Vector::default(),
            rotation: UnitQuaternion::default(),
            shape: ColliderShapeDef::Plane {
                offset_along_normal: 0.0,
            },
        }],
        dt,
    );
    let query_pipeline = query_world.as_query_pipeline(QueryFilter::only_fixed());
    let mut target_xz_cache: HashMap<Owner, Vec2> = HashMap::new();
    let view_db = ctx.as_read_only().db;
    for mut move_intent in ctx.db.move_intent_tbl().iter() {
        let owner = move_intent.owner;
        let Some(target_xz) = move_intent
            .data
            .target_position_with_cache(&view_db, &mut target_xz_cache)
        else {
            move_intent.delete(ctx);
            continue;
        };

        let Some(mut owner_transform) = Transform::find(ctx, owner) else {
            move_intent.delete(ctx);
            continue;
        };
        let Some(mut movement_state) = MovementState::find(ctx, owner) else {
            move_intent.delete(ctx);
            continue;
        };

        let target_xz: Vector2<f32> = target_xz.into();
        let current_xz: Vector2<f32> = owner_transform.data.translation.xz().into();

        let mut movement_state_dirty = false;
        // Capping vertical velocity reduces writes to DB. No need to fall faster than terminal velocity.
        if !movement_state.grounded && movement_state.vertical_velocity < TERMINAL_VELOCITY {
            movement_state.vertical_velocity += GRAVITY * dt;
            movement_state_dirty = true;
        }

        let Some(speed) = SecondaryStats::find(&ctx.as_read_only(), owner)
            .map(|secondary_stats| secondary_stats.data.movement_speed)
        else {
            log::error!("Failed to find secondary stats for entity {}", owner);
            move_intent.delete(ctx);
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
        owner_transform.update(ctx, owner_transform.data);

        let new_cell_id = encode_cell_id(
            owner_transform.data.translation.x,
            owner_transform.data.translation.z,
        );
        if movement_state.cell_id != new_cell_id {
            movement_state.cell_id = new_cell_id;
            movement_state_dirty = true;
        }
        // Update the MovementState when it has changed
        if !movement_state.grounded || correction.grounded != movement_state.grounded {
            movement_state.grounded = correction.grounded;
            movement_state_dirty = true;
        }
        if movement_state_dirty {
            ctx.db.movement_state_tbl().owner().update(movement_state);
        }

        if is_at_target_planar(owner_transform.data.translation.xz().into(), target_xz) {
            if let MoveIntentData::Point(_) = move_intent.data {
                move_intent.delete(ctx);
            } else if let MoveIntentData::Path(ref mut path) = move_intent.data {
                if !path.is_empty() {
                    path.remove(0);
                }
                if path.is_empty() {
                    move_intent.delete(ctx);
                } else {
                    ctx.db.move_intent_tbl().owner().update(move_intent);
                }
            }
        }
    }

    // Delete the old scheduled reducer and create a new one instead up updating an "ScheduledAt::Interval"
    // version because of an engine bug that considers time elapsed in reducer as part of the Interval.
    // https://discord.com/channels/1037340874172014652/1455222615747858534
    reschedule(ctx, timer);

    Ok(())
}
