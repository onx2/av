use crate::{schema::*, types::*};
use shared::utils::encode_cell_id;
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
pub fn enter_world(ctx: &ReducerContext) -> Result<(), String> {
    let Some(mut player) = ctx.db.player().identity().find(ctx.sender) else {
        return Err("No player found!".into());
    };

    if let Some(_) = player.actor_id {
        return Err("Player is already in world".into());
    }

    // Rebuild actor from persisted Player state.
    let actor = ctx.db.actor().insert(Actor {
        id: 0,
        kind: ActorKind::Player(player.identity),
        translation: player.translation,
        yaw: player.yaw,
        capsule_radius: player.capsule_radius,
        capsule_half_height: player.capsule_half_height,
        movement_speed: player.movement_speed,
        move_intent: MoveIntent::None,
        grounded: false,
        is_player: true,
        grounded_grace_steps: 0,
        cell_id: encode_cell_id(player.translation.x, player.translation.z),
    });

    // Link back Player -> Actor.
    player.actor_id = Some(actor.id);
    ctx.db.player().identity().update(player);
    Ok(())
}
