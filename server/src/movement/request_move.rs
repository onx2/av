use crate::{active_character_tbl, move_intent_tbl, transform_tbl, MoveIntentData, MoveIntentRow};
use nalgebra::Vector2;
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

    let current: Vector2<f32> = transform_data.data.translation.xz().into();

    // Should we ignore this request based on our current intent?
    if let Some(current_intent) = MoveIntentRow::find(ctx, active_character.owner) {
        // Rate limit move requests to 20/sec
        if let Some(dur) = ctx.timestamp.duration_since(current_intent.sent_at) {
            if dur.as_millis() < 50 {
                return Err("Rate limited".into());
            }
        }
        let should_ignore = match (current_intent.data, &intent) {
            // Already chasing this actor
            (MoveIntentData::Actor(id_a), MoveIntentData::Actor(id_b)) => id_a == *id_b,
            // Current point is already close enough to this new point
            (MoveIntentData::Point(p_a), MoveIntentData::Point(p_b)) => {
                is_move_too_close(p_a.into(), (*p_b).into())
            }
            _ => false,
        };

        if should_ignore {
            return Ok(());
        }
    };

    // Is this new intent valid?
    match &intent {
        MoveIntentData::Point(point) => {
            if is_move_too_close(current, (*point).into()) {
                return Err("Distance from current position too close".into());
            }
        }
        MoveIntentData::Path(p) => {
            if p.into_iter().any(|x| is_move_too_far(current, (*x).into())) {
                return Err("Distance from current position too far".into());
            }
        }
        MoveIntentData::Actor(owner) => {
            let Some(target) = ctx.db.transform_tbl().owner().find(owner) else {
                return Err("Unable to find target for move intent".into());
            };

            // Only check if the actor is too far because this can be used to follow, even when close.
            if is_move_too_far(current, target.data.translation.xz().into()) {
                return Err("Distance from current position too far".into());
            }
        }
    }

    MoveIntentRow::upsert(ctx, active_character.owner, intent);
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
