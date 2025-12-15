//! Connection lifecycle reducers.
//!
//! These reducers handle Player row creation and cleanup for clients as they
//! connect to, and disconnect from, the authoritative SpacetimeDB module.
//!
//! Behavior
//! - On connect:
//!   - Ensure a `Player` row exists for the caller's identity.
//!   - Clear any dangling `actor_id` to start from a consistent state.
//!   - Seed sensible defaults if the row is newly created.
//! - On disconnect:
//!   - If a live `Actor` exists, persist its authoritative state back to the
//!     corresponding `Player` row (transform, collider, speed, grounded).
//!   - Despawn the `Actor` row and clear `player.actor_id`.
//!
//! Determinism
//! - These reducers only perform data-layer operations (no physics).
//! - State transitions are explicit and logged for traceability.

use crate::schema::*;
use spacetimedb::{ReducerContext, Table};

/// Fired when a client connects to the module.
///
/// Ensures a `Player` row exists and clears dangling `actor_id`. If no row
/// exists, a default one is created so subsequent reducers can rely on it.
///
/// This reducer does not spawn an `Actor`. That is handled by `enter_world`.
#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);

    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        // Clear any dangling live actor reference on reconnect.
        ctx.db.player().identity().update(Player {
            actor_id: None,
            ..player
        });
    } else {
        // Seed a new player with sensible defaults so we can rebuild an actor later.
        ctx.db.player().insert(Player {
            identity: ctx.sender,
            actor_id: None,
            translation: DbVec3::new(0.0, 3.85, 0.0),
            rotation: DbQuat::default(),
            scale: DbVec3::ONE,
            capsule_radius: 0.35,
            capsule_half_height: 0.75,
            movement_speed: 5.0,
        });
    }
}

/// Fired when a client disconnects from the module.
///
/// If a live `Actor` exists, this reducer persists its current state back to
/// the `Player` row and despawns the actor. It then clears `player.actor_id`.
#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    log::info!("Client disconnected: {:?}", ctx.sender);

    let Some(mut player) = ctx.db.player().identity().find(ctx.sender) else {
        return;
    };

    let Some(actor_id) = player.actor_id else {
        return;
    };

    if let Some(actor) = ctx.db.actor().id().find(actor_id) {
        // Persist authoritative actor state to Player.
        player.translation = actor.translation;
        player.rotation = actor.rotation;
        player.scale = actor.scale;
        player.capsule_radius = actor.capsule_radius;
        player.capsule_half_height = actor.capsule_half_height;
        player.movement_speed = actor.movement_speed;

        // Despawn the actor and clear the link.
        ctx.db.actor().id().delete(actor.id);
    }

    player.actor_id = None;
    ctx.db.player().identity().update(player);
}
