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
use spacetimedb::{ReducerContext, Table};

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
    let acceptance = (actor.capsule_radius * 2.0).max(0.0);

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
        MoveIntent::Path(mut path) => {
            // Empty path is a no-op; clear intent.
            if path.is_empty() {
                actor.move_intent = MoveIntent::None;
            } else {
                // Remove any leading waypoints that are already within acceptance.
                loop {
                    let wp = path[0];
                    if within_acceptance(actor.translation, wp, acceptance) {
                        path.remove(0);
                        if path.is_empty() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if path.is_empty() {
                    actor.move_intent = MoveIntent::None;
                } else {
                    // Reject if the first waypoint is too far away.
                    let wp = path[0];
                    if !within_movement_range(actor.translation, wp, DEFAULT_MAX_INTENT_DISTANCE) {
                        return Err("First waypoint too far from current position".to_string());
                    }
                    actor.move_intent = MoveIntent::Path(path);
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

/// Enter the world: (re)create the caller's live actor from their persisted Player state.
///
/// Behavior:
/// - Validates a Player row exists for the caller.
/// - Rejects if the caller already has a live actor.
/// - Spawns a new Actor row seeded from the Player's persisted fields
///   (transform, capsule dimensions, movement speed, grounded).
/// - Sets `player.actor_id = Some(actor.id)`.
///
/// Determinism:
/// - Actor creation uses persisted state only. No randomization occurs here.
/// - The kinematic controller will immediately process this actor on the next tick.
#[spacetimedb::reducer]
pub fn enter_world(ctx: &ReducerContext) {
    let Some(mut player) = ctx.db.player().identity().find(ctx.sender) else {
        // If a Player row doesn't exist, this likely indicates that `identity_connected`
        // did not run, or the DB was reset. For robustness, create a minimal default.
        let default = Player {
            identity: ctx.sender,
            actor_id: None,
            translation: DbVec3::new(0.0, 3.85, 0.0),
            rotation: DbQuat::default(),
            scale: DbVec3::ONE,
            capsule_radius: 0.35,
            capsule_half_height: 0.75,
            movement_speed: 5.0,
            grounded: false,
        };
        let _ = ctx.db.player().insert(default);
        return;
    };

    if let Some(_) = player.actor_id {
        // Already in world—ignore duplicate requests.
        return;
    }

    // Rebuild actor from persisted Player state.
    let actor = ctx.db.actor().insert(Actor {
        id: 0,
        kind: ActorKind::Player(player.identity),
        translation: player.translation,
        rotation: player.rotation,
        scale: player.scale,
        capsule_radius: player.capsule_radius,
        capsule_half_height: player.capsule_half_height,
        movement_speed: player.movement_speed,
        grounded: player.grounded,
        move_intent: MoveIntent::None,
    });

    // Link back Player -> Actor.
    player.actor_id = Some(actor.id);
    ctx.db.player().identity().update(player);
}

/// Leave the world: persist the caller's actor state and despawn the live actor.
///
/// Behavior:
/// - Validates a Player row exists for the caller.
/// - If no live actor exists, this is a no-op.
/// - Otherwise:
///   - Persists current Actor state back to Player (transform, capsule, speed, grounded).
///   - Deletes the Actor row.
///   - Clears `player.actor_id`.
///
/// Determinism:
/// - This reducer is purely a state transition and data persistence step; it does
///   not run the physics controller.
#[spacetimedb::reducer]
pub fn leave_world(ctx: &ReducerContext) {
    let Some(mut player) = ctx.db.player().identity().find(ctx.sender) else {
        return;
    };

    let Some(actor_id) = player.actor_id else {
        // No live actor; nothing to do.
        return;
    };

    let Some(actor) = ctx.db.actor().id().find(actor_id) else {
        // Inconsistent state; clear the dangling id.
        player.actor_id = None;
        ctx.db.player().identity().update(player);
        return;
    };

    // Persist actor state back to Player.
    player.translation = actor.translation;
    player.rotation = actor.rotation;
    player.scale = actor.scale;
    player.capsule_radius = actor.capsule_radius;
    player.capsule_half_height = actor.capsule_half_height;
    player.movement_speed = actor.movement_speed;
    player.grounded = actor.grounded;

    // Despawn actor and clear link.
    ctx.db.actor().id().delete(actor.id);
    player.actor_id = None;

    // Save updated Player row.
    ctx.db.player().identity().update(player);
}
