use crate::schema::*;
use spacetimedb::ReducerContext;

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
    player.yaw = actor.yaw;
    player.capsule_radius = actor.capsule_radius;
    player.capsule_half_height = actor.capsule_half_height;
    player.movement_speed = actor.movement_speed;

    // Despawn actor and clear link.
    ctx.db.actor().id().delete(actor.id);
    player.actor_id = None;

    // Save updated Player row.
    ctx.db.player().identity().update(player);
}
