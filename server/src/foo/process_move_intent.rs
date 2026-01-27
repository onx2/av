use super::{
    actor_tbl, move_intent_tbl, movement_state_tbl, transform_tbl, ComputedStat, DataTable,
    MoveIntentData, MovementSpeed, Transform,
};
use nalgebra::{UnitQuaternion, Vector2, Vector3};
use rapier3d::control::{CharacterAutostep, KinematicCharacterController};
use shared::{encode_cell_id, get_desired_delta, is_at_target_planar, yaw_from_xz};
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

    log::info!("Delta Time: {:?}", dt);

    let kcc = KinematicCharacterController {
        autostep: Some(CharacterAutostep {
            include_dynamic_bodies: false,
            ..CharacterAutostep::default()
        }),
        ..KinematicCharacterController::default()
    };

    // For every movement intent
    for mut move_intent in ctx.db.move_intent_tbl().iter() {
        let owner = move_intent.owner;

        let Some(mut owner_transform) = Transform::find(ctx, owner) else {
            move_intent.delete(ctx);
            continue;
        };
        let Some(mut actor) = ctx.db.actor_tbl().owner().find(owner) else {
            move_intent.delete(ctx);
            continue;
        };
        let Some(mut movement_state) = ctx.db.movement_state_tbl().owner().find(owner) else {
            move_intent.delete(ctx);
            continue;
        };
        let Some(target_xz) = move_intent.data.target_position(&ctx.as_read_only().db) else {
            move_intent.delete(ctx);
            continue;
        };

        // Convert to nalgebra for math operations.
        let target_xz: Vector2<f32> = target_xz.into();
        let current_xz: Vector2<f32> = owner_transform.data.translation.xz().into();

        if !movement_state.data.grounded {
            movement_state.data.vertical_velocity += -9.81 * dt;
        }
        let speed = MovementSpeed::compute(&ctx.as_read_only().db, owner)
            .map(|ms| ms.value)
            .unwrap_or(0.0);
        let direction = (target_xz - current_xz)
            .try_normalize(0.0)
            .unwrap_or_default();
        let desired_delta = get_desired_delta(
            current_xz,
            target_xz,
            speed,
            movement_state.data.vertical_velocity,
            movement_state.data.grounded,
            dt,
        );

        // TODO collision detection:
        // let correction = kcc.move_shape(
        //     dt,
        //     &query_pipeline,
        //     &Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
        //     &owner_transform.data.translation.into(),
        //     desired_delta.into(),
        //     |_| {},
        // );
        // owner_transform.data.translation.x += correction.translation.x;
        // owner_transform.data.translation.y += correction.translation.y;
        // owner_transform.data.translation.z += correction.translation.z;
        if let Some(yaw) = yaw_from_xz(direction) {
            owner_transform.data.rotation =
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw).into();
        }
        let new_xz: Vector2<f32> = owner_transform.data.translation.xz().into();

        let new_cell_id = encode_cell_id(
            owner_transform.data.translation.x,
            owner_transform.data.translation.z,
        );
        if actor.cell_id != new_cell_id {
            actor.cell_id = new_cell_id;
            ctx.db.actor_tbl().owner().update(actor);
        }

        ctx.db.transform_tbl().owner().update(owner_transform);

        // Update the MovementState when it has changed
        // if !movement_state.data.grounded || correction.grounded != movement_state.data.grounded {
        //     movement_state.data.grounded = correction.grounded;
        //     ctx.db.movement_state_tbl().owner().update(movement_state);
        // }

        // Should we finish the movement intent?
        if is_at_target_planar(new_xz, target_xz) {
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
    ctx.db
        .movement_tick_timer()
        .scheduled_id()
        .delete(timer.scheduled_id);
    ctx.db.movement_tick_timer().insert(ProcessMoveIntentTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Time(ctx.timestamp + TimeDuration::from_micros(50_000)),
        last_tick: ctx.timestamp,
    });

    Ok(())
}
