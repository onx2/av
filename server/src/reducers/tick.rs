use crate::schema::actor;
use crate::types::{ActorKind, DbVec3, MoveIntent};
use crate::{
    schema::kcc_settings,
    tick_timer,
    utils::{get_fixed_delta_time, get_variable_delta_time, has_support_within},
    world::world_query_world,
    TickTimer,
};
use spacetimedb::{ReducerContext, Table};

/// Clear point intents after this many consecutive fixed-steps without meaningful progress.
const STUCK_CLEAR_STEPS: u8 = 20;

/// Squared progress epsilon (meters^2) used to determine "no progress".
/// This is intentionally small; we use the stuck counter to avoid false positives.
const STUCK_PROGRESS_EPS_SQ: f32 = 1.0e-6;

/// When a monster is stuck, immediately re-roll a new wander target instead of idling.
/// This makes them recover without waiting for the next tick loop to notice `MoveIntent::None`.
const MONSTER_STUCK_RETARGET: bool = true;

// Use Rapier types/macros through the shared crate to keep dependency versions unified.
use shared::{
    rapier_world::rapier3d::prelude::*,
    utils::{yaw_from_xz, UtilMath},
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
            // If this is one of our stress-test "fake remotes", make it wander forever by
            // assigning a new random-ish target whenever it is idle.
            //
            // We intentionally keep this dependency-free (no RNG crate) and use a simple
            // deterministic-ish hash of (timestamp, actor id) so each tick yields different
            // scatter without having to store extra per-actor state.
            if matches!(actor.kind, ActorKind::Monster(_))
                && matches!(actor.move_intent, MoveIntent::None)
            {
                let (tx, tz) = wander_target(ctx, actor.id, actor.translation);
                actor.move_intent = MoveIntent::Point(DbVec3::new(tx, actor.translation.y, tz));
            }

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

            // Stuck detection: remember starting distance to target for this step.
            let dist_sq_before = dist_sq;

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
            if let Some(yaw) = yaw_from_xz(planar.x, planar.z) {
                actor.yaw = yaw;
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
            if has_point_intent && dist_sq_before <= kcc.point_acceptance_radius_sq {
                actor.move_intent = MoveIntent::None;
                actor.stuck_steps = 0;
            } else if has_point_intent {
                // Stuck detection: if we didn't meaningfully reduce distance-to-target this step,
                // increment stuck counter; otherwise reset it.
                let ndx = target_x - actor.translation.x;
                let ndz = target_z - actor.translation.z;
                let dist_sq_after = ndx.sq() + ndz.sq();

                let made_progress = dist_sq_after + STUCK_PROGRESS_EPS_SQ < dist_sq_before;
                if made_progress {
                    actor.stuck_steps = 0;
                } else {
                    actor.stuck_steps = actor.stuck_steps.saturating_add(1);

                    if actor.stuck_steps >= STUCK_CLEAR_STEPS {
                        // If this is a wandering monster, immediately pick a new destination
                        // so it visibly "unsticks" and keeps roaming without idling.
                        if MONSTER_STUCK_RETARGET && matches!(actor.kind, ActorKind::Monster(_)) {
                            let (tx, tz) = wander_target(ctx, actor.id, actor.translation);
                            actor.move_intent =
                                MoveIntent::Point(DbVec3::new(tx, actor.translation.y, tz));
                        } else {
                            actor.move_intent = MoveIntent::None;
                        }

                        actor.stuck_steps = 0;
                    }
                }
            } else {
                actor.stuck_steps = 0;
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

/// Compute a new wander target around the given position for a fake remote actor.
///
/// Returns (x, z). Y is preserved by the caller.
///
/// Tuning:
/// - Keeps targets reasonably close so you see constant motion near the player.
/// - Uses a deterministic-ish pseudo-random source derived from timestamp + actor id.
fn wander_target(ctx: &ReducerContext, actor_id: u64, from: DbVec3) -> (f32, f32) {
    // Wander in a ring around current position.
    let wander_radius_min: f32 = 2.0;
    let wander_radius_max: f32 = 10.0;

    let r0 = prand01(ctx, actor_id, 0);
    let r1 = prand01(ctx, actor_id, 1);

    let theta = r0 * core::f32::consts::TAU;
    let radius = lerp(wander_radius_min, wander_radius_max, r1.sqrt());

    (from.x + theta.cos() * radius, from.z + theta.sin() * radius)
}

/// Deterministic-ish pseudo-random float in [0, 1), derived from reducer timestamp and actor id.
///
/// This is intentionally simple and dependency-free; it's only for stress-test wandering.
fn prand01(ctx: &ReducerContext, actor_id: u64, salt: u32) -> f32 {
    // Hash Debug strings so we don't depend on Timestamp/Identity internals.
    let ts_str = format!("{:?}", ctx.timestamp);

    // 32-bit FNV-1a over timestamp, then mix in actor_id + salt.
    let mut h: u32 = 2166136261u32;
    for b in ts_str.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619u32);
    }

    // Mix in actor id (split to avoid relying on endianness).
    let lo = actor_id as u32;
    let hi = (actor_id >> 32) as u32;

    h ^= lo.wrapping_mul(0x9E37_79B9);
    h = h.rotate_left(13).wrapping_mul(0x85EB_CA6B);
    h ^= hi.wrapping_mul(0xC2B2_AE35);
    h = h.rotate_left(17).wrapping_mul(0x27D4_EB2D);

    // Mix in salt.
    h ^= salt.wrapping_mul(0x1656_67B1);
    h ^= h >> 16;

    // Map to [0,1) using 24 bits of precision.
    let mantissa = (h >> 8) as u32;
    (mantissa as f32) / ((1u32 << 24) as f32)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
