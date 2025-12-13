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
    model::{within_movement_range, yaw_to_db_quat, DEFAULT_MAX_INTENT_DISTANCE},
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

/// Consolidated helper: planar move toward a target with capsule acceptance,
/// horizontal KCC, yaw update, and ground snap. Returns whether the acceptance
/// boundary was reached this step (finished).
fn move_toward_planar_target(
    ctx: &ReducerContext,
    actor: &mut crate::schema::Actor,
    start_pos: na::Vector3<f32>,
    capsule: shared::collision::CapsuleSpec,
    target: DbVec3,
    delta_time_seconds: f32,
) -> bool {
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
    // Desired planar translation; vertical handled via snap.
    let desired = na::Vector3::new(plan.desired_translation.x, 0.0, plan.desired_translation.z);

    // Update yaw if moving.
    let dx = desired.x;
    let dz = desired.z;
    if dx * dx + dz * dz > f32::EPSILON {
        let yaw = (-dx).atan2(-dz);
        actor.rotation = yaw_to_db_quat(yaw);
    }

    // Horizontal KCC, then ground snap.
    let moved = physics::kcc_horizontal_step_with_defaults(ctx, start_pos, desired, capsule);
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

    plan.finished
}

// Helper to resolve a common target position from the current intent.
// Returns (Some(target), false) when the target is valid,
// (None, true) when the target should be cleared (e.g., missing actor),
// and (None, false) when there is simply no target (intent is None or empty Path).
fn resolve_target(ctx: &ReducerContext, actor: &crate::schema::Actor) -> (Option<DbVec3>, bool) {
    match &actor.move_intent {
        MoveIntent::Point(p) => (Some(*p), false),
        MoveIntent::Path(path) => {
            if path.is_empty() {
                (None, false)
            } else {
                (Some(path[0]), false)
            }
        }
        MoveIntent::Actor(target_id) => {
            if let Some(target) = ctx.db.actor().id().find(*target_id) {
                (Some(target.translation), false)
            } else {
                (None, true)
            }
        }
        MoveIntent::None => (None, false),
    }
}

// Helper to update/clear movement intent after a movement step.
// - Point: clear on finish
// - Path: pop the first waypoint on finish; clear when empty
// - Actor: keep chasing (higher-level behavior should decide when to stop)
fn update_intent_progress(actor: &mut crate::schema::Actor, finished: bool) {
    match &mut actor.move_intent {
        MoveIntent::Point(_) => {
            if finished {
                actor.move_intent = MoveIntent::None;
            }
        }
        MoveIntent::Path(path) => {
            if finished && !path.is_empty() {
                // Paths are expected to be short; O(N) remove(0) is acceptable here.
                path.remove(0);
            }
            if path.is_empty() {
                actor.move_intent = MoveIntent::None;
            }
        }
        MoveIntent::Actor(_) => {
            // Keep chasing; higher-level system (combat) decides when to stop.
        }
        MoveIntent::None => {}
    }
}

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
    let delta_time_seconds =
        crate::model::delta_seconds_with_rate(ctx.timestamp, timer.last_tick, TICK_RATE);

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

        // Resolve target from intent and decide whether to clear it this frame.
        let (target_opt, should_clear_intent) = resolve_target(ctx, &actor);

        // Decide whether to do a planar movement step this frame.
        // We only move planarly if grounded, have a target, and it's within range.
        let mut perform_planar_move = false;
        if actor.grounded {
            if let Some(t) = target_opt {
                if within_movement_range(actor.translation, t, DEFAULT_MAX_INTENT_DISTANCE) {
                    perform_planar_move = true;
                }
            }
        }

        // Execute physics once: either planar move or gravity-only.
        let mut movement_finished = false;
        if perform_planar_move {
            movement_finished = move_toward_planar_target(
                ctx,
                &mut actor,
                start_pos,
                capsule,
                target_opt.unwrap(),
                delta_time_seconds,
            );
        } else {
            let (final_pos, grounded) = physics::gravity_step_with_defaults(
                ctx,
                start_pos,
                capsule,
                FALL_SPEED_MPS,
                delta_time_seconds,
            );
            actor.translation = DbVec3::from(final_pos);
            actor.grounded = grounded;
        }

        // Intent cleanup and progress update (save-once pattern).
        if should_clear_intent {
            actor.move_intent = MoveIntent::None;
        } else if perform_planar_move {
            update_intent_progress(&mut actor, movement_finished);
        } else if !actor.grounded {
            // Design choice: clear while falling (matches earlier behavior)
            actor.move_intent = MoveIntent::None;
        }

        // Save once at the end of the iteration.
        ctx.db.actor().id().update(actor);
    }

    Ok(())
}
