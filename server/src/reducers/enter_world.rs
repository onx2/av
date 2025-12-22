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
/// - Initializes movement state directly on the Actor (MovementData has been removed).
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

    // Movement state is stored directly on the live Actor (MovementData table removed).

    // Rebuild actor from persisted Player state.
    //
    // Movement state is initialized directly on the Actor:
    // - intent starts as Idle(now)
    // - grounded starts false (KCC will set it on first tick)
    // - should_move starts true so the actor gets processed at least once (gravity/grounding)
    let actor = ctx.db.actor().insert(Actor {
        id: 0,
        primary_stats_id: player.primary_stats_id,
        secondary_stats_id: player.secondary_stats_id,
        vital_stats_id: player.vital_stats_id,
        transform_data_id: player.transform_data_id,
        // kind: ActorKind::Player(player.identity),
        is_player: true,
        identity: Some(player.identity),
        capsule_radius: player.capsule_radius,
        capsule_half_height: player.capsule_half_height,
        move_intent: MoveIntent::Idle(ctx.timestamp.to_micros_since_unix_epoch().max(0) as u64),
        grounded: false,
        grounded_grace_steps: 0,
        should_move: true,
        // `TransformData.translation` is mixed precision (`DbVec3i16`):
        // - x/z are already meters (f32)
        // - y is quantized (i16, `Y_QUANTIZE_STEP_M`) but is not needed for cell id
        cell_id: encode_cell_id(transform_data.translation.x, transform_data.translation.z),
    });

    // Link back Player -> Actor.
    player.actor_id = Some(actor.id);
    ctx.db.player().identity().update(player);
    Ok(())
}
