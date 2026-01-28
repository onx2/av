use super::{
    actor_tbl, move_intent_tbl, ComputedStat, DataTable, MoveIntentData, MovementSpeed,
    MovementState, Transform,
};
use nalgebra::{UnitQuaternion, Vector, Vector2, Vector3};
use rapier3d::{
    control::{CharacterAutostep, KinematicCharacterController},
    prelude::{Capsule, QueryFilter},
};
use shared::{
    constants::GRAVITY, encode_cell_id, get_desired_delta, is_at_target_planar,
    utils::build_static_query_world, yaw_from_xz, ColliderShapeDef, WorldStaticDef,
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
        scheduled_at: ScheduleAt::Time(ctx.timestamp + TimeDuration::from_micros(50_000)),
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

    let view_db = ctx.as_read_only().db;
    for mut move_intent in ctx.db.move_intent_tbl().iter() {
        let owner = move_intent.owner;
        let Some(target_xz) = move_intent.data.target_position(&view_db) else {
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
        let Some(mut actor) = ctx.db.actor_tbl().owner().find(owner) else {
            move_intent.delete(ctx);
            continue;
        };

        let target_xz: Vector2<f32> = target_xz.into();
        let current_xz: Vector2<f32> = owner_transform.data.translation.xz().into();

        if !movement_state.data.grounded {
            movement_state.data.vertical_velocity += GRAVITY * dt;
        }
        let speed = MovementSpeed::compute(&view_db, owner)
            .map(|ms| ms.value)
            .unwrap_or(0.0);
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
            &Capsule::new_y(0.85, 0.3),
            &owner_transform.data.into(),
            get_desired_delta(
                current_xz,
                target_xz,
                speed,
                movement_state.data.vertical_velocity,
                movement_state.data.grounded,
                dt,
            ),
            |_| {},
        );
        owner_transform.data.translation.x += correction.translation.x;
        owner_transform.data.translation.y += correction.translation.y;
        owner_transform.data.translation.z += correction.translation.z;

        let new_cell_id = encode_cell_id(
            owner_transform.data.translation.x,
            owner_transform.data.translation.z,
        );
        if actor.cell_id != new_cell_id {
            actor.cell_id = new_cell_id;
            ctx.db.actor_tbl().owner().update(actor);
        }
        owner_transform.update(ctx, owner_transform.data);

        // Update the MovementState when it has changed
        if !movement_state.data.grounded || correction.grounded != movement_state.data.grounded {
            movement_state.data.grounded = correction.grounded;
            movement_state.update(ctx, movement_state.data);
        }

        // Should we finish the movement intent?
        if is_at_target_planar(owner_transform.data.translation.xz().into(), target_xz) {
            match &mut move_intent.data {
                MoveIntentData::Point(_) => {
                    move_intent.delete(ctx);
                }
                MoveIntentData::Path(path) => {
                    if !path.is_empty() {
                        path.remove(0);
                    }
                    if path.is_empty() {
                        move_intent.delete(ctx);
                    } else {
                        ctx.db.move_intent_tbl().owner().update(move_intent);
                    }
                }
                _ => {}
            }
        }
    }

    // Delete the old scheduled reducer and create a new one instead up updating an "ScheduledAt::Interval"
    // version because of an engine bug that considers time elapsed in reducer as part of the Interval.
    // https://discord.com/channels/1037340874172014652/1455222615747858534
    reschedule(ctx, timer);

    Ok(())
}
