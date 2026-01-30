use crate::{active_character_tbl, move_intent_tbl, transform_tbl, MoveIntent, MoveIntentData};
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

    let current: na::Vector2<f32> = transform_data.data.translation.xz().into();
    let Some(target) = intent.target_position(&ctx.as_read_only().db).map(|t| {
        let t: na::Vector2<f32> = t.into();
        t
    }) else {
        return Err("Unable to find target position".into());
    };

    // Basic validation, are we currently too close or too far from the next target position in the intent we want to go?
    if is_move_too_close(&current, &target) {
        return Err("Distance from current position too close".into());
    }
    // TODO: validate each point to see if they are too far.
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

// match (&source_actor.move_intent, &intent) {
//     // 1. Idling Check
//     (MoveIntent::None, MoveIntent::None) => {
//         return Err("Already idling".into());
//     }

//     // 2. History Check: Is the new point too close to the old intent point?
//     (MoveIntent::Point(old), MoveIntent::Point(new))
//         if is_move_too_close(&old.into(), &new.into()) =>
//     {
//         return Err("Distance from last point too close".into());
//     }

//     // 3. Path Validation: Complexity check
//     (_, MoveIntent::Path(p)) if p.len() > MAX_INTENT_PATH_LEN => {
//         return Err("Path is too complex".into());
//     }

//     // 4. Path Validation: Range check (are any points too far?)
//     (_, MoveIntent::Path(p)) if p.iter().any(|x| is_move_too_far(&current, &x.into())) => {
//         return Err("Distance from current position too far".into());
//     }

//     // 5. Point Validation: Minimum movement check (from current position)
//     (_, MoveIntent::Point(p)) if is_move_too_close(&current, &p.into()) => {
//         return Err("Distance from current position too close".into());
//     }

//     _ => {
//         // Movement state now lives directly on `Actor`.
//         //
//         // Keep `should_move` consistent with the movement tick behavior:
//         // - should_move if we have a non-idle intent, OR if we're airborne (gravity needs processing).
//         let should_move = intent != MoveIntent::None || !source_actor.grounded;

//         if source_actor.move_intent != intent || source_actor.should_move != should_move {
//             ctx.db.actor().id().update(Actor {
//                 move_intent: intent,
//                 should_move,
//                 ..source_actor
//             });
//         }

//         Ok(())
//     }
// }
