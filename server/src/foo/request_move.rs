use super::{active_character_tbl, move_intent_tbl, transform_tbl, MoveIntent, MoveIntentData};
use nalgebra as na;
use shared::utils::{is_move_too_close, is_move_too_far};
use spacetimedb::{reducer, ReducerContext};

/// Request a movement intent for the player's active character, removing any previous request.
#[reducer]
pub fn request_move(ctx: &ReducerContext, intent: MoveIntentData) -> Result<(), String> {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return Err("Unable to find active character".into());
    };

    let Some(transform_data) = ctx.db.transform_tbl().owner().find(active_character.owner) else {
        return Err("Unable to find transform for the active character".into());
    };

    let current: na::Vector3<f32> = transform_data.data.translation.into();
    let target: na::Vector3<f32> = intent
        .target_position(&ctx.as_read_only().db)
        .map(|t| t.extend(current.y).into())
        .unwrap_or(current);

    // Basic validation, are we currently too close or too far from the next target position in the intent we want to go?
    if is_move_too_close(&current, &target) {
        return Err("Distance from current position too close".into());
    }
    if is_move_too_far(&current, &target) {
        return Err("Distance from current position too far".into());
    }

    MoveIntent::upsert(ctx, active_character.owner, intent);
    Ok(())
}

#[reducer]
pub fn cancel_move(ctx: &ReducerContext) -> Result<(), String> {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return Err("Unable to find active character".into());
    };

    ctx.db
        .move_intent_tbl()
        .owner()
        .delete(active_character.owner);
    Ok(())
}
