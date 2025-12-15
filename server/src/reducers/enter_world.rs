use crate::schema::*;
use spacetimedb::{ReducerContext, Table};

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
        move_intent: MoveIntent::None,
        grounded: false,
    });

    // Link back Player -> Actor.
    player.actor_id = Some(actor.id);
    ctx.db.player().identity().update(player);
}
