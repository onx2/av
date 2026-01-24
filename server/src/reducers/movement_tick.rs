use crate::{
    schema::{actor, secondary_stats, transform_data, world_static},
    types::MoveIntent,
    utils::{get_fixed_delta_time, get_variable_delta_time},
    world::row_to_def,
};
use rapier3d::{
    control::{CharacterAutostep, KinematicCharacterController},
    prelude::{Capsule, QueryFilter},
};
use shared::{
    encode_cell_id, get_desired_delta, is_at_target_planar, utils::build_static_query_world,
    yaw_from_xz, yaw_to_u16,
};
use spacetimedb::{ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};

#[spacetimedb::table(
    name = movement_tick_timer,
    scheduled(movement_tick_reducer)
)]
pub struct MovementTickTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
    pub last_tick: Timestamp,
}

pub fn init_movement_tick(ctx: &ReducerContext) {
    let interval = TimeDuration::from_micros(50_000); // 20HZ
    ctx.db.movement_tick_timer().scheduled_id().delete(1);
    ctx.db.movement_tick_timer().insert(MovementTickTimer {
        scheduled_id: 1,
        scheduled_at: ScheduleAt::Interval(interval),
        last_tick: ctx.timestamp,
    });
}

#[spacetimedb::reducer]
fn movement_tick_reducer(ctx: &ReducerContext, mut timer: MovementTickTimer) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("`movement_tick_reducer` may not be invoked by clients.".into());
    }

    // Compute real elapsed time since last tick; fallback to scheduled fixed dt.
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);

    // ---------------------------------------------------------------------------------------------------------
    // Build the KinematicCharacterController
    //  ---------------------------------------------------------------------------------------------------------
    let kcc = KinematicCharacterController {
        autostep: Some(CharacterAutostep {
            include_dynamic_bodies: false,
            ..CharacterAutostep::default()
        }),
        ..KinematicCharacterController::default()
    };

    let query_world =
        build_static_query_world(ctx.db.world_static().iter().map(row_to_def), real_dt);
    let query_pipeline = query_world.as_query_pipeline(QueryFilter::only_fixed());

    // ---------------------------------------------------------------------------------------------------------
    // Move each actor and update
    // ---------------------------------------------------------------------------------------------------------
    for mut actor in ctx.db.actor().should_move().filter(true) {
        let Some(mut transform) = ctx.db.transform_data().id().find(actor.transform_data_id) else {
            continue;
        };

        let current_planar = transform.translation.vec2_xz();
        let target_planar = match &actor.move_intent {
            MoveIntent::Point(p) => p.vec2_xz(),
            _ => current_planar,
        };

        let speed = ctx
            .db
            .secondary_stats()
            .id()
            .find(actor.secondary_stats_id)
            .map(|s| s.movement_speed)
            .unwrap_or_default();

        let direction = (target_planar - current_planar)
            .try_normalize(0.0)
            .unwrap_or_default();
        let desired_delta = get_desired_delta(
            current_planar,
            target_planar,
            speed,
            actor.grounded,
            real_dt,
        );

        // KCC move against the static collision world.
        let correction = kcc.move_shape(
            real_dt,
            &query_pipeline,
            &Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
            &transform.translation.into(),
            desired_delta.into(),
            |_| {},
        );
        transform.translation.x += correction.translation.x;
        transform.translation.y += correction.translation.y;
        transform.translation.z += correction.translation.z;

        // Yaw based on desired planar (XZ) movement this tick, not the corrected because we want to have the
        // actor look like they are facing their intended direction not the actual direction they are moving.
        if let Some(yaw) = yaw_from_xz([direction.x, direction.y]) {
            transform.yaw = yaw_to_u16(yaw);
        }

        if is_at_target_planar(current_planar, target_planar) {
            actor.move_intent = MoveIntent::None;
        }
        actor.should_move = actor.move_intent != MoveIntent::None || !correction.grounded;
        actor.grounded = correction.grounded;
        actor.cell_id = encode_cell_id(transform.translation.x, transform.translation.z);
        ctx.db.actor().id().update(actor);
        ctx.db.transform_data().id().update(transform);
    }

    // Persist timer state.
    timer.last_tick = ctx.timestamp;
    ctx.db.movement_tick_timer().scheduled_id().update(timer);

    Ok(())
}
