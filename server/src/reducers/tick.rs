use crate::schema::{actor, MoveIntent};
use crate::utils::has_support_within;
use crate::{
    schema::kcc_settings,
    tick_timer,
    utils::{get_fixed_delta_time, get_variable_delta_time},
    world::world_query_world,
    TickTimer,
};
use spacetimedb::{ReducerContext, Table};

// Use Rapier types/macros through the shared crate to keep dependency versions unified.
use shared::{
    rapier_world::rapier3d::prelude::*,
    utils::{rotation_from_xz, UtilMath},
};

// In rapier3d 0.31.0 the character controller types live under `rapier3d::control`.
use shared::rapier_world::rapier3d::control::{
    CharacterAutostep, CharacterLength, KinematicCharacterController,
};

/// Safety cap to avoid spending unbounded time catching up after long stalls.
const MAX_STEPS_PER_TICK: u32 = 5;

#[spacetimedb::reducer]
pub fn tick(ctx: &ReducerContext, mut timer: TickTimer) -> Result<(), String> {
    // Only the server (module identity) may invoke the scheduled reducer.
    if ctx.sender != ctx.identity() {
        return Err("`tick` may not be invoked by clients.".into());
    }

    let Some(kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("Missing kcc_settings row (expected id = 1)".into());
    };
    // Fixed timestep.
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);

    // Real time elapsed since last tick; used to advance the accumulator.
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);

    // Accumulate real time; drain it in fixed-size steps.
    timer.time_accumulator += real_dt;

    // Cache the immutable world query state once per reducer invocation.
    let world = world_query_world(ctx);

    let controller = KinematicCharacterController {
        offset: CharacterLength::Absolute(kcc.offset),
        max_slope_climb_angle: kcc.max_slope_climb_deg.to_radians(),
        min_slope_slide_angle: kcc.min_slope_slide_deg.to_radians(),
        snap_to_ground: None,
        autostep: Some(CharacterAutostep {
            max_height: CharacterLength::Absolute(kcc.autostep_max_height),
            min_width: CharacterLength::Absolute(kcc.autostep_min_width),
            include_dynamic_bodies: false,
        }),
        slide: kcc.slide,
        normal_nudge_factor: kcc.normal_nudge_factor,
        ..KinematicCharacterController::default()
    };

    let mut steps_ran: u32 = 0;
    while timer.time_accumulator >= fixed_dt {
        // Borrowed query pipeline view for this step (Rapier 0.31).
        let query_pipeline = world.query_pipeline(QueryFilter::default());

        // Process all actors.
        for mut actor in ctx.db.actor().iter() {
            // Determine desired planar movement for this fixed step.
            // For now, handle only MoveIntent::Point; other intents result in no planar motion.
            //
            // IMPORTANT: Copy target coordinates out into locals so we don't hold a borrow of
            // `actor.translation` across the rest of this block (we mutate translation later).
            let (target_x, target_z, has_point_intent) = match &actor.move_intent {
                MoveIntent::Point(p) => (p.x, p.z, true),
                _ => (actor.translation.x, actor.translation.z, false),
            };

            // Compute intended planar direction toward the target.
            let dx = target_x - actor.translation.x;
            let dz = target_z - actor.translation.z;

            // Avoid sqrt unless we actually need a direction.
            let dist_sq = dx.sq() + dz.sq();

            // Planar step length.
            let max_step = if has_point_intent {
                actor.movement_speed.max(0.0) * fixed_dt
            } else {
                0.0
            };

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
            if let Some(quat) = rotation_from_xz(planar.x, planar.z) {
                actor.rotation = quat.into();
            }

            // Apply downward bias always; apply fall speed only if we were airborne last step.
            let down_bias = -kcc.grounded_down_bias_mps * fixed_dt;

            // `actor.grounded` is persisted and represents the grounded state from the previous fixed step.
            // This gives us the desired 1-tick lag without any global in-memory cache.
            let gravity = f32::from(!actor.grounded) * (-kcc.fall_speed_mps * fixed_dt);

            let corrected = controller.move_shape(
                fixed_dt,
                &query_pipeline,
                &Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
                &Isometry::translation(
                    actor.translation.x,
                    actor.translation.y,
                    actor.translation.z,
                ),
                vector![planar.x, down_bias + gravity, planar.z],
                |_| {},
            );

            // Apply corrected movement.
            actor.translation.x += corrected.translation.x;
            actor.translation.y += corrected.translation.y;
            actor.translation.z += corrected.translation.z;

            // Persist grounded for the next fixed step.
            if corrected.grounded {
                actor.grounded = true;
                actor.grounded_grace_steps = 8;
            } else if actor.grounded_grace_steps > 0 {
                let supported = has_support_within(
                    &query_pipeline,
                    &actor,
                    kcc.hard_airborne_probe_distance,
                    kcc.max_slope_climb_deg.to_radians().cos(),
                );

                if supported {
                    actor.grounded_grace_steps -= 1;
                } else {
                    actor.grounded_grace_steps = 0;
                    actor.grounded = false;
                }
            } else {
                actor.grounded = false;
            }

            // Clear MoveIntent::Point when within the acceptance radius (planar).
            // TODO: Acceptance radius should be computed differently
            if has_point_intent && dist_sq <= kcc.point_acceptance_radius_sq {
                actor.move_intent = MoveIntent::None;
            }

            ctx.db.actor().id().update(actor);
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
    ctx.db.tick_timer().scheduled_id().update(timer);

    Ok(())
}
