use crate::{schema::*, types::MoveIntent};
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

    let Some(transform_data) = ctx.db.transform_data().id().find(player.transform_data_id) else {
        return Err("No transform found!".into());
    };

    // Rebuild actor from persisted Player state.
    //
    // Movement state now lives directly on `Actor` (the old `MovementData` table is removed),
    // so seed the movement-related fields from sensible defaults.
    let actor = ctx.db.actor().insert(Actor {
        id: 0,
        primary_stats_id: player.primary_stats_id,
        secondary_stats_id: player.secondary_stats_id,
        vital_stats_id: player.vital_stats_id,
        transform_data_id: player.transform_data_id,
        identity: Some(player.identity),
        capsule_radius: player.capsule_radius,
        capsule_half_height: player.capsule_half_height,
        should_move: true,
        move_intent: MoveIntent::None,
        grounded: false,
        cell_id: encode_cell_id(transform_data.translation.x, transform_data.translation.z),
    });

    // Link back Player -> Actor.
    player.actor_id = Some(actor.id);
    ctx.db.player().identity().update(player);
    Ok(())
}
