use crate::{schema::*, types::*};
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
        let transform_data = ctx.db.transform_data().insert(TransformData {
            id: 0,
            translation: DbVec3::new(0.0, 3.85, 0.0),
            yaw: 0.0,
        });
        let movement_data = ctx.db.movement_data().insert(MovementData {
            id: 0,
            move_intent: MoveIntent::None,
            grounded: false,
            should_move: true,
            grounded_grace_steps: 0,
        });
        let primary_stats = ctx.db.primary_stats().insert(PrimaryStats {
            id: 0,
            strength: 10,
            dexterity: 10,
            fortitude: 10,
            intelligence: 10,
            piety: 10,
        });
        let secondary_stats = ctx.db.secondary_stats().insert(SecondaryStats {
            id: 0,
            movement_speed: 5.0,
            max_health: 100,
            max_mana: 100,
            max_stamina: 100,
        });
        let vital_stats = ctx.db.vital_stats().insert(VitalStats {
            id: 0,
            health: 100,
            mana: 100,
            stamina: 100,
        });
        // Seed a new player with sensible defaults so we can rebuild an actor later.
        ctx.db.player().insert(Player {
            identity: ctx.sender,
            actor_id: None,
            transform_data_id: transform_data.id,
            movement_data_id: movement_data.id,
            capsule_radius: 0.35,
            capsule_half_height: 0.75,
            primary_stats_id: primary_stats.id,
            secondary_stats_id: secondary_stats.id,
            vital_stats_id: vital_stats.id,
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
        // this shouldn't really be necesary but doing it anyway for now...
        player.capsule_radius = actor.capsule_radius;
        player.capsule_half_height = actor.capsule_half_height;

        // Despawn the actor and clear the link.
        ctx.db.actor().id().delete(actor.id);
    }

    ctx.db.actor_in_aoi().identity().delete(player.identity);
    player.actor_id = None;
    ctx.db.player().identity().update(player);
}
