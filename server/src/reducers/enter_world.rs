use crate::schema::*;
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

    let Some(movement_data) = ctx.db.movement_data().id().find(player.movement_data_id) else {
        return Err("No movement data found!".into());
    };

    // Rebuild actor from persisted Player state.
    let actor = ctx.db.actor().insert(Actor {
        id: 0,
        primary_stats_id: player.primary_stats_id,
        secondary_stats_id: player.secondary_stats_id,
        vital_stats_id: player.vital_stats_id,
        transform_data_id: player.transform_data_id,
        movement_data_id: movement_data.id,
        // kind: ActorKind::Player(player.identity),
        is_player: true,
        identity: Some(player.identity),
        // translation: player.translation,
        // yaw: player.yaw,
        capsule_radius: player.capsule_radius,
        capsule_half_height: player.capsule_half_height,
        // Keep the duplicated flag consistent with the persisted MovementData row.
        should_move: movement_data.should_move,
        // `TransformData.translation` is mixed precision (`DbVec3i16`):
        // - x/z are already meters (f32)
        // - y is quantized (i16, 0.1m) but is not needed for cell id
        cell_id: encode_cell_id(transform_data.translation.x, transform_data.translation.z),
    });

    // Link back Player -> Actor.
    player.actor_id = Some(actor.id);
    ctx.db.player().identity().update(player);
    Ok(())
}
