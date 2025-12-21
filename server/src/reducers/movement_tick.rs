use crate::{
    schema::{actor, kcc_settings, movement_data, secondary_stats, transform_data},
    types::MoveIntent,
    utils::{get_fixed_delta_time, get_variable_delta_time, has_support_within},
    world::{get_kcc, get_rapier_world},
};
use rapier3d::prelude::*;
use shared::utils::{encode_cell_id, yaw_from_xz, UtilMath};
use spacetimedb::*;

/// Safety cap to avoid spending unbounded time catching up after long stalls.
const MAX_STEPS_PER_TICK: u32 = 3;

#[table(name = movement_tick_timer, scheduled(movement_tick_reducer))]
pub struct MovementTickTimer {
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

pub fn init(ctx: &ReducerContext) {
    let tick_interval = TimeDuration::from_micros(1_000_000 / 30);
    ctx.db.movement_tick_timer().scheduled_id().delete(1);
    ctx.db.movement_tick_timer().insert(MovementTickTimer {
        scheduled_id: 1,
        scheduled_at: spacetimedb::ScheduleAt::Interval(tick_interval),
        last_tick: ctx.timestamp,
        time_accumulator: 0.,
    });
}

#[spacetimedb::reducer]
pub fn movement_tick_reducer(
    ctx: &ReducerContext,
    mut timer: MovementTickTimer,
) -> Result<(), String> {
    // Only the server (module identity) may invoke the scheduled reducer.
    if ctx.sender != ctx.identity() {
        return Err("`tick` may not be invoked by clients.".into());
    }

    // Fixed timestep.
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);

    // Real time elapsed since last tick; used to advance the accumulator.
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);

    // Accumulate real time; drain it in fixed-size steps.
    timer.time_accumulator += real_dt;
    let Some(kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("`tick` couldn't find kcc settings.".into());
    };
    // Cache the immutable world query state once per reducer invocation.
    let world = get_rapier_world(ctx);
    let controller = get_kcc(ctx);

    let mut steps_ran: u32 = 0;
    while timer.time_accumulator >= fixed_dt {
        // Borrowed query pipeline view for this step
        let query_pipeline = world.query_pipeline(QueryFilter::default());

        // Process all actors.
        for mut movement_data in ctx.db.movement_data().should_move().filter(true) {
            let Some(mut actor) = ctx.db.actor().movement_data_id().find(movement_data.id) else {
                continue;
            };
            let Some(mut transform_data) =
                ctx.db.transform_data().id().find(actor.transform_data_id)
            else {
                continue;
            };

            let capsule_half_height = actor.capsule_half_height;
            let capsule_radius = actor.capsule_radius;
            // Determine desired planar movement for this fixed step.
            // For now, handle only MoveIntent::Point; other intents result in no planar motion.
            let (target_x, target_z, has_point_intent) = match &movement_data.move_intent {
                MoveIntent::Point(p) => (p.x, p.z, true),
                _ => (
                    transform_data.translation.x,
                    transform_data.translation.z,
                    false,
                ),
            };

            // Compute intended planar direction toward the target.
            let dx = target_x - transform_data.translation.x;
            let dz = target_z - transform_data.translation.z;

            let secondary_stats = ctx
                .db
                .secondary_stats()
                .id()
                .find(actor.secondary_stats_id)
                .unwrap_or_default();

            // Planar step length.
            let max_step = if has_point_intent {
                secondary_stats.movement_speed * fixed_dt
            } else {
                0.0
            };

            // Avoid sqrt unless we actually need a direction.
            let dist_sq = dx.sq() + dz.sq();

            let planar = if dist_sq > 1.0e-12 && max_step > 0.0 {
                let dist = dist_sq.sqrt();
                let inv_dist = 1.0 / dist;
                let step = max_step.min(dist);
                vector![dx * inv_dist * step, 0.0, dz * inv_dist * step]
            } else {
                vector![0.0, 0.0, 0.0]
            };

            // Update yaw based on *intent* direction (not post-collision motion).
            // This looks more natural when sliding along walls.
            if let Some(yaw) = yaw_from_xz(planar.x, planar.z) {
                transform_data.yaw = yaw;
            }

            // Apply downward bias always; apply fall speed only if we were airborne last step.
            let down_bias = -kcc.grounded_down_bias_mps * fixed_dt;

            // `actor.grounded` is persisted and represents the grounded state from the previous fixed step.
            // This gives us the desired 1-tick lag without any global in-memory cache.
            let gravity = f32::from(!movement_data.grounded) * (-kcc.fall_speed_mps * fixed_dt);

            let corrected = controller.move_shape(
                fixed_dt,
                &query_pipeline,
                &Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
                &Isometry::translation(
                    transform_data.translation.x,
                    transform_data.translation.y,
                    transform_data.translation.z,
                ),
                vector![planar.x, down_bias + gravity, planar.z],
                |_| {},
            );

            // Apply corrected movement.
            transform_data.translation.x += corrected.translation.x;
            transform_data.translation.y += corrected.translation.y;
            transform_data.translation.z += corrected.translation.z;

            let new_cell_id =
                encode_cell_id(transform_data.translation.x, transform_data.translation.z);
            if new_cell_id != actor.cell_id {
                actor.cell_id = new_cell_id;
                ctx.db.actor().id().update(actor);
            }

            // Persist grounded for the next fixed step.
            if corrected.grounded {
                movement_data.grounded = true;
                movement_data.grounded_grace_steps = 8;
            } else if movement_data.grounded_grace_steps > 0 {
                let supported = has_support_within(
                    &query_pipeline,
                    &transform_data.translation,
                    capsule_half_height,
                    capsule_radius,
                    kcc.hard_airborne_probe_distance,
                    kcc.max_slope_climb_deg.to_radians().cos(),
                );

                if supported {
                    movement_data.grounded_grace_steps -= 1;
                } else {
                    movement_data.grounded_grace_steps = 0;
                    movement_data.grounded = false;
                }
            } else {
                movement_data.grounded = false;
            }

            // Clear MoveIntent::Point when within the acceptance radius (planar).
            // TODO: Acceptance radius should be computed differently
            if has_point_intent && dist_sq <= kcc.point_acceptance_radius_sq {
                movement_data.move_intent = MoveIntent::None;
            }

            movement_data.should_move =
                movement_data.move_intent != MoveIntent::None || !movement_data.grounded;
            ctx.db.transform_data().id().update(transform_data);
            ctx.db.movement_data().id().update(movement_data);
        }

        // Consume fixed time step.
        timer.time_accumulator -= fixed_dt;
        steps_ran += 1;

        if steps_ran >= MAX_STEPS_PER_TICK {
            // Prevent runaway catch-up loops.
            timer.time_accumulator = 0.0;
            break;
        }
    }

    // Persist timer state.
    timer.last_tick = ctx.timestamp;
    ctx.db.movement_tick_timer().scheduled_id().update(timer);

    Ok(())
}
