use crate::schema::*;
use spacetimedb::ReducerContext;

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

    actor.move_intent = intent;
    ctx.db.actor().id().update(actor);

    Ok(())
}
