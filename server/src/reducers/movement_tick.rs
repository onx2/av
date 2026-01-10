use crate::{
    schema::{actor, kcc_settings, secondary_stats, transform_data, world_static},
    types::MoveIntent,
    utils::{get_fixed_delta_time, get_variable_delta_time},
    world::row_to_def,
};
use nalgebra::vector;
use rapier3d::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    prelude::Capsule,
};
use shared::{
    compute_desired_translation, encode_cell_id, is_at_target_planar,
    utils::{build_static_query_world, has_support_within},
    yaw_from_xz, yaw_to_u8,
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
    let Some(kcc_settings) = ctx.db.kcc_settings().id().find(1) else {
        return Err("`movement_tick_reducer` couldn't find kcc settings.".into());
    };
    let kcc = KinematicCharacterController {
        offset: CharacterLength::Absolute(kcc_settings.offset),
        max_slope_climb_angle: kcc_settings.max_slope_climb_deg.to_radians(),
        min_slope_slide_angle: kcc_settings.min_slope_slide_deg.to_radians(),
        snap_to_ground: Some(CharacterLength::Absolute(0.3)),
        autostep: Some(CharacterAutostep {
            max_height: CharacterLength::Absolute(kcc_settings.autostep_max_height),
            min_width: CharacterLength::Absolute(kcc_settings.autostep_min_width),
            include_dynamic_bodies: false,
        }),
        slide: kcc_settings.slide,
        normal_nudge_factor: kcc_settings.normal_nudge_factor,
        ..KinematicCharacterController::default()
    };

    let query_world = build_static_query_world(
        ctx.db.world_static().iter().map(row_to_def).collect(),
        real_dt,
    );
    let query_pipeline = query_world.as_query_pipeline();

    // ---------------------------------------------------------------------------------------------------------
    // Move each actor and update
    // ---------------------------------------------------------------------------------------------------------
    for mut actor in ctx.db.actor().should_move().filter(true) {
        let Some(mut transform) = ctx.db.transform_data().id().find(actor.transform_data_id) else {
            continue;
        };

        let current_planar = [transform.translation.x, transform.translation.z];
        let target_planar = match &actor.move_intent {
            MoveIntent::Point(p) => [p.x, p.z],
            _ => current_planar,
        };

        let supported = if actor.grounded {
            true
        } else {
            has_support_within(
                &query_pipeline,
                &[
                    transform.translation.x,
                    transform.translation.y,
                    transform.translation.z,
                ],
                actor.capsule_half_height,
                actor.capsule_radius,
                0.75,
                kcc_settings.max_slope_climb_deg.to_radians().cos(),
            )
        };

        let Some(desired_translation) = ctx
            .db
            .secondary_stats()
            .id()
            .find(actor.secondary_stats_id)
            .map(|row| {
                let translation = compute_desired_translation(
                    current_planar,
                    target_planar,
                    row.movement_speed,
                    real_dt,
                    supported,
                    kcc_settings.grounded_down_bias_mps,
                    kcc_settings.fall_speed_mps,
                    0.35,
                );
                vector![translation[0], translation[1], translation[2]]
            })
        else {
            log::error!(
                "Unable to find the secondary stats of actor: {:?}",
                actor.id
            );
            continue;
        };

        // Yaw based on planar (XZ) movement this tick.
        let desired_planar = desired_translation.xz();
        if let Some(yaw) = yaw_from_xz(&desired_planar) {
            transform.yaw = yaw_to_u8(yaw);
        }

        // KCC move against the static collision world.
        let corrected = kcc.move_shape(
            real_dt,
            &query_pipeline,
            &Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
            &transform.translation.into(),
            desired_translation,
            |_| {},
        );

        // Apply corrected movement
        transform.translation.x += corrected.translation.x;
        transform.translation.y += corrected.translation.y;
        transform.translation.z += corrected.translation.z;

        let mut actor_dirty = false;
        // Clear move intent after the actual move, once we're within acceptance radius.
        if let MoveIntent::Point(p) = actor.move_intent {
            let new_xyz = [
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
            ];
            let target_xyz = [p.x, p.y, p.z];
            if is_at_target_planar(new_xyz, target_xyz, kcc_settings.point_acceptance_radius_sq) {
                actor.move_intent = MoveIntent::None;
                actor_dirty = true;
            }
        }

        // Update cell id on actor when crossing cells (cell encoding expects meters).
        let new_cell_id = encode_cell_id(transform.translation.x, transform.translation.z);
        if new_cell_id != actor.cell_id {
            actor.cell_id = new_cell_id;
            actor_dirty = true;
        }

        // Only update grounded state when it has changed
        if actor.grounded != corrected.grounded {
            actor.grounded = corrected.grounded;
            actor_dirty = true;
        }

        // Actor should move when it has a movement intent or is not grounded.
        let new_should_move = actor.move_intent != MoveIntent::None || !actor.grounded;
        if actor.should_move != new_should_move {
            actor.should_move = new_should_move;
            actor_dirty = true;
        }

        ctx.db.transform_data().id().update(transform);
        if actor_dirty == true {
            ctx.db.actor().id().update(actor);
        }
    }

    // Persist timer state.
    timer.last_tick = ctx.timestamp;
    ctx.db.movement_tick_timer().scheduled_id().update(timer);

    Ok(())
}
