//! Tick reducer: authoritative per-frame simulation pipeline.
//!
//! Responsibilities
//! - Compute delta time deterministically from the scheduled tick timer.
//! - Drive the kinematic character controller (KCC) for every actor:
//!   - If grounded: compute intent-based horizontal step → sweep-and-slide → snap.
//!   - If falling: cancel horizontal motion and apply gravity-only step → snap.
//! - Write authoritative transforms and grounded state back to the database.
//!
//! Design notes
//! - Broad- and narrow-phase collision queries are delegated to the shared `collision` crate,
//!   via thin wrappers in `crate::physics`.
//! - World statics are immutable and cached with a lock-free initializer in `crate::world`.
//! - Intent-level motion (acceptance radius, speed × dt) is computed with `shared::motion`.
//!
//! Determinism
//! - All state that affects the simulation is read, advanced, and written within this reducer.
//! - No asynchronous work or external randomness is used.

use crate::{
    model::yaw_to_db_quat,
    physics,
    schema::{actor, DbVec3, MoveIntent},
    tick_timer, TickTimer,
};
use nalgebra as na;
use shared::{
    collision::settings::{SNAP_HOVER_HEIGHT, SNAP_MAX_DISTANCE},
    motion,
};
use spacetimedb::{ReducerContext, Table, TimeDuration};

/// Target tick rate and derived fallback delta for defensive timing.
///
/// The reducer uses the timestamp difference between the current invocation
/// and the last tick for delta time. In the unlikely event the delta cannot
/// be computed, it falls back to this target cadence.
const TICK_RATE: i64 = 60;
const DELTA_MICRO_SECS: i64 = 1_000_000 / TICK_RATE;

/// Constant downward fall speed (m/s) when airborne.
///
/// This simple model is deterministic and cost-effective for a kinematic
/// controller. If you later want true acceleration, you can integrate
/// `-GRAVITY_MPS2 * dt` into a vertical velocity term and clamp it.
const FALL_SPEED_MPS: f32 = -10.0;

/// Authoritative per-frame simulation.
///
/// Pipeline (per actor):
/// - If grounded:
///   - Compute desired planar motion (speed × dt, with capsule-based acceptance).
///   - Sweep-and-slide horizontally (KCC).
///   - Snap to ground (downward cast) to keep hover height.
/// - If airborne:
///   - Cancel horizontal intent (if any).
///   - Apply gravity-only step (vertical sweep) and snap.
///
/// All transforms and grounded state are written back to the DB.
#[spacetimedb::reducer]
pub fn tick(ctx: &ReducerContext, mut timer: TickTimer) -> Result<(), String> {
    // Only the server (module identity) may invoke the scheduled reducer.
    if ctx.sender != ctx.identity() {
        return Err("`tick` may not be invoked by clients.".into());
    }

    // Compute delta time, update timer state.
    let delta_time_seconds = ctx
        .timestamp
        .time_duration_since(timer.last_tick)
        .unwrap_or(TimeDuration::from_micros(DELTA_MICRO_SECS))
        .to_micros() as f32
        / 1_000_000.0;

    timer.last_tick = ctx.timestamp;
    ctx.db.tick_timer().scheduled_id().update(timer);

    // Process all actors every tick (gravity applies even without movement intent).
    for mut actor in ctx.db.actor().iter() {
        // Shared capsule spec used by the KCC this frame.
        let capsule = shared::collision::CapsuleSpec {
            radius: actor.capsule_radius,
            half_height: actor.capsule_half_height,
        };

        // Actor's current center as a vector.
        let start_pos = na::Vector3::new(
            actor.translation.x,
            actor.translation.y,
            actor.translation.z,
        );

        match actor.move_intent {
            MoveIntent::None => {
                // Gravity-only step when there is no intent.
                let (final_pos, grounded) = physics::gravity_step_with_defaults(
                    ctx,
                    start_pos,
                    capsule,
                    FALL_SPEED_MPS,
                    delta_time_seconds,
                );

                // Authoritative write-back.
                actor.translation = DbVec3::from(final_pos);
                actor.grounded = grounded;
                ctx.db.actor().id().update(actor);
            }

            MoveIntent::Point(target) => {
                // If airborne, cancel intent and apply gravity-only step.
                if !actor.grounded {
                    actor.move_intent = MoveIntent::None;

                    let (final_pos, grounded) = physics::gravity_step_with_defaults(
                        ctx,
                        start_pos,
                        capsule,
                        FALL_SPEED_MPS,
                        delta_time_seconds,
                    );

                    actor.translation = DbVec3::from(final_pos);
                    actor.grounded = grounded;
                    ctx.db.actor().id().update(actor);
                    continue;
                }

                // 1) Compute intent-level desired planar motion with capsule-based acceptance.
                let current_p = na::Point3::new(
                    actor.translation.x,
                    actor.translation.y,
                    actor.translation.z,
                );
                let target_p = na::Point3::new(target.x, target.y, target.z);
                let plan = motion::compute_desired_with_capsule_acceptance(
                    current_p,
                    // Keep navigation planar by clamping target.y to current.y.
                    na::Point3::new(target_p.x, current_p.y, target_p.z),
                    actor.movement_speed,
                    delta_time_seconds,
                    actor.capsule_radius,
                );

                // Zero vertical intent. Gravity/snap handle Y.
                let desired =
                    na::Vector3::new(plan.desired_translation.x, 0.0, plan.desired_translation.z);

                // 2) Update yaw from intended horizontal direction (if any).
                let dx = desired.x;
                let dz = desired.z;
                if dx * dx + dz * dz > f32::EPSILON {
                    let yaw = (-dx).atan2(-dz);
                    actor.rotation = yaw_to_db_quat(yaw);
                }

                // 3) Sweep-and-slide horizontally (KCC).
                let moved =
                    physics::kcc_horizontal_step_with_defaults(ctx, start_pos, desired, capsule);

                // 4) Ground snap to keep hover height.
                let (snapped, grounded) = physics::snap_to_ground(
                    ctx,
                    moved.end_pos,
                    capsule,
                    SNAP_MAX_DISTANCE,
                    SNAP_HOVER_HEIGHT,
                );

                // Authoritative write-back.
                actor.translation = DbVec3::from(snapped);
                actor.grounded = grounded;

                // Clear intent on acceptance hit.
                if plan.finished {
                    actor.move_intent = MoveIntent::None;
                }

                ctx.db.actor().id().update(actor);
            }

            // TODO: Extend with Path/Actor following; default to gravity-only while unsupported.
            _ => {
                actor.move_intent = MoveIntent::None;
                let (final_pos, grounded) = physics::gravity_step_with_defaults(
                    ctx,
                    start_pos,
                    capsule,
                    FALL_SPEED_MPS,
                    delta_time_seconds,
                );
                actor.translation = DbVec3::from(final_pos);
                actor.grounded = grounded;
                ctx.db.actor().id().update(actor);
            }
        }
    }

    Ok(())
}
