//! Movement-related reducers: request a move, enter world, and leave world.
//!
//! These reducers implement the authoritative control flow around an actor's
//! lifecycle and movement intent:
//! - `enter_world`: (re)create an actor from the persisted Player state.
//! - `leave_world`: persist the current Actor state back to Player and despawn the actor.
//! - `request_move`: set a movement intent, with guardrails (e.g., cannot set while falling).
//!
//! Design notes
//! - The server is authoritative. These reducers validate the caller identity and
//!   interact with the database exclusively via SpacetimeDB row reads/writes.
//! - Persisting the actor state on leave/disconnect enables deterministic rejoin:
//!   `enter_world` reconstructs the actor using the Player's saved transform,
//!   collider dimensions, and movement speed.
//! - Movement intent is consumed by the `tick` reducer each frame. Horizontal
//!   motion is canceled while airborne (falling), so this reducer rejects new
//!   intents when `grounded == false`.

use crate::model::{within_acceptance, within_movement_range, DEFAULT_MAX_INTENT_DISTANCE};
use crate::schema::*;
use spacetimedb::ReducerContext;

/// Request or update the movement intent for the caller's live actor.
///
/// Behavior:
/// - Validates the caller has a Player row and a live Actor row.
/// - Rejects requests while airborne (falling) to avoid desync with the KCC
///   policy that cancels horizontal motion mid-air.
/// - Otherwise sets `actor.move_intent = intent`.
///
/// Errors:
/// - Returns an error string if the Player or Actor rows cannot be found,
///   or if the actor is currently falling.
///
/// Determinism:
/// - Intent is a data input to the kinematic controller. The `tick` reducer
///   consumes it deterministically and clears it when acceptance is reached.
#[spacetimedb::reducer]
pub fn request_move(ctx: &ReducerContext, intent: MoveIntent) -> Result<(), String> {
    // Locate the Player row for this identity.
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return Err("Player not found".to_string());
    };

    // Ensure the caller currently has a live actor.
    let Some(actor_id) = player.actor_id else {
        return Err("Actor not found".to_string());
    };

    let Some(mut actor) = ctx.db.actor().id().find(actor_id) else {
        return Err("Actor not found".to_string());
    };

    // Guard: do not accept horizontal intents while airborne.
    if !actor.grounded {
        return Err("Actor is falling; cannot set move intent right now".to_string());
    }

    // Precompute planar acceptance radius (capsule + small buffer).
    let acceptance = shared::motion::acceptance_from_capsule(actor.capsule_radius);

    // Validate intent and normalize where applicable (e.g., trim close waypoints).
    match intent {
        MoveIntent::Point(p) => {
            // Reject if the destination is already within acceptance.
            if within_acceptance(actor.translation, p, acceptance) {
                return Err("Destination too close to current position".to_string());
            }
            // Reject if the destination is too far away.
            if !within_movement_range(actor.translation, p, DEFAULT_MAX_INTENT_DISTANCE) {
                return Err("Destination too far from current position".to_string());
            }
            actor.move_intent = MoveIntent::Point(p);
        }
        MoveIntent::Path(path) => {
            // Empty path is a no-op; clear intent.
            if path.is_empty() {
                actor.move_intent = MoveIntent::None;
            } else {
                // Build a validated path ensuring:
                // - Every waypoint is outside acceptance from the actor and previous waypoint.
                // - Every waypoint is within max range from the actor.
                // - Consecutive waypoints are not too far apart.
                let mut validated: Vec<DbVec3> = Vec::with_capacity(path.len());
                let mut prev = actor.translation;

                for wp in path.into_iter() {
                    // Skip waypoints within acceptance of the actor or the previous accepted waypoint.
                    if within_acceptance(actor.translation, wp, acceptance) {
                        continue;
                    }
                    if within_acceptance(prev, wp, acceptance) {
                        continue;
                    }

                    // Enforce max range from the actor and between consecutive points.
                    if !within_movement_range(actor.translation, wp, DEFAULT_MAX_INTENT_DISTANCE) {
                        return Err("Waypoint too far from actor".to_string());
                    }
                    if !within_movement_range(prev, wp, DEFAULT_MAX_INTENT_DISTANCE) {
                        return Err("Consecutive waypoints too far apart".to_string());
                    }

                    validated.push(wp);
                    prev = wp;
                }

                if validated.is_empty() {
                    actor.move_intent = MoveIntent::None;
                } else {
                    actor.move_intent = MoveIntent::Path(validated);
                }
            }
        }
        MoveIntent::Actor(target_id) => {
            // Disallow chasing self and require a valid target actor.
            if target_id == actor.id {
                return Err("Cannot set follow intent to self".to_string());
            }
            let Some(target_actor) = ctx.db.actor().id().find(target_id) else {
                return Err("Target actor not found".to_string());
            };
            // Reject if the target actor is too far away (planar).
            if !within_movement_range(
                actor.translation,
                target_actor.translation,
                DEFAULT_MAX_INTENT_DISTANCE,
            ) {
                return Err("Target actor too far from current position".to_string());
            }
            actor.move_intent = MoveIntent::Actor(target_id);
        }
        MoveIntent::None => {
            actor.move_intent = MoveIntent::None;
        }
    }

    // Persist changes.
    ctx.db.actor().id().update(actor);

    Ok(())
}
