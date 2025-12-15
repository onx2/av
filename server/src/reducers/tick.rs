use crate::{
    model::{delta_seconds_with_rate, get_delta_time, yaw_to_db_quat},
    schema::{actor, MoveIntent},
    tick_timer,
    world::{world_accel, world_statics},
    TickTimer,
};
use shared::{
    collision::{settings::acceptance_from_capsule, CapsuleSpec},
    step_movement,
};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
pub fn tick(ctx: &ReducerContext, mut timer: TickTimer) -> Result<(), String> {
    // 1. Guard against client calls (Correctly done already)
    if ctx.sender != ctx.identity() {
        return Err("`tick` may not be invoked by clients.".into());
    }

    // --- TIME CALCULATION ---
    let fixed_dt: f32 = get_delta_time(timer.scheduled_at);
    let real_dt: f32 = ctx
        .timestamp
        .time_duration_since(timer.last_tick)
        .unwrap_or(spacetimedb::TimeDuration::from_micros(1_000_000 / 60))
        .to_micros() as f32
        / 1_000_000.0;

    // You can remove this logging line once debugging is finished,
    // as it clutters the logs and the true delta time is FIXED_DT.
    log::info!("Delta Times: {:?} | {:?}", fixed_dt, real_dt);

    // 2. ACCUMULATE TIME
    // Use the real elapsed time to push the simulation clock forward.
    timer.time_accumulator += real_dt;

    // --- THE FIXED-STEP CATCH-UP LOOP ---
    let mut steps_ran: u32 = 0;

    let statics = world_statics(ctx);
    let accel = world_accel(ctx);

    log::info!("Statics: {:?}", statics.len());

    // 3. Loop: Run the deterministic steps until the accumulator is drained below FIXED_DT
    while timer.time_accumulator >= fixed_dt {
        // Process all actors every tick (gravity applies even without movement intent).
        for mut actor in ctx.db.actor().iter() {
            // Build capsule spec (field names here must match your schema).
            let capsule = CapsuleSpec {
                radius: actor.capsule_radius,
                half_height: actor.capsule_half_height,
            };

            let acceptance = acceptance_from_capsule(actor.capsule_radius);
            let target = match actor.move_intent {
                MoveIntent::Point(point) => point,
                _ => actor.translation,
            };
            let res = step_movement(
                statics,
                accel,
                capsule,
                actor.translation.into(),
                target.into(),
                actor.movement_speed,
                fixed_dt,
                acceptance,
            );

            actor.translation = res.new_translation.into();
            actor.grounded = res.is_grounded;

            if let Some(yaw_quat) = res.new_rotation {
                actor.rotation = yaw_quat.into();
            }

            // Save once at the end of the iteration.
            ctx.db.actor().id().update(actor);
        }

        // -----------------------------------------------------------------

        // 4. CONSUME TIME
        // Subtract exactly the fixed time step from the accumulator.
        timer.time_accumulator -= fixed_dt;
        steps_ran += 1;

        // OPTIONAL: Safety break for extreme lag (prevents infinite loop/crash)
        if steps_ran > 5 {
            log::warn!("Server severely lagged! Ran max steps. Dumping accumulator time.");
            timer.time_accumulator = 0.0;
            break;
        }
    }

    // 5. Cleanup and Persist State
    timer.last_tick = ctx.timestamp;

    // Persist the updated timer row (with the new last_tick AND the remaining time_accumulator)
    ctx.db.tick_timer().scheduled_id().update(timer);

    log::info!("Steps Ran: {}", steps_ran); // Helpful to see the catch-up in action

    Ok(())
}
