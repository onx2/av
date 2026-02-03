use crate::{character_instance_tbl, movement_state_tbl, transform_tbl, MoveIntentData};
use nalgebra::Vector2;
use shared::utils::{is_move_too_close, is_move_too_far};
use spacetimedb::{reducer, ReducerContext};

/// Request a movement intent for the player's active character.
///
/// New approach:
/// - `movement_state_tbl.move_intent` stores the current intent.
/// - `movement_state_tbl.should_move` is kept consistent with the movement tick:
///     `should_move = move_intent.is_some() || !grounded`
#[reducer]
pub fn request_move(ctx: &ReducerContext, intent: MoveIntentData) -> Result<(), String> {
    let Some(ci) = ctx.db.character_instance_tbl().identity().find(ctx.sender) else {
        log::error!("Unable to find active character");
        return Err("Unable to find active character".into());
    };

    let Some(transform_row) = ctx.db.transform_tbl().actor_id().find(ci.actor_id) else {
        log::error!("Unable to find transform for the active character");
        return Err("Unable to find transform for the active character".into());
    };

    let current: Vector2<f32> = transform_row.data.translation.xz().into();

    // Load movement state we will update. (Move intents now live here.)
    let Some(mut movement_state) = ctx.db.movement_state_tbl().actor_id().find(ci.actor_id) else {
        log::error!("Unable to find movement state for the active character");
        return Err("Unable to find movement state for the active character".into());
    };

    // Should we ignore this request based on our current intent?
    if let Some(current_intent) = movement_state.move_intent.as_ref() {
        let should_ignore = match (current_intent, &intent) {
            // Already chasing this actor
            (MoveIntentData::Actor(id_a), MoveIntentData::Actor(id_b)) => id_a == id_b,
            // Current point is already close enough to this new point
            (MoveIntentData::Point(p_a), MoveIntentData::Point(p_b)) => {
                is_move_too_close((*p_a).into(), (*p_b).into())
            }
            // Note: for Path vs Path / Path vs Point comparisons we don't ignore by default.
            _ => false,
        };

        if should_ignore {
            log::info!("Ignoring duplicate move intent");
            return Ok(());
        }
    }

    // Is this new intent valid?
    match &intent {
        MoveIntentData::Point(point) => {
            if is_move_too_close(current, (*point).into()) {
                log::info!(
                    "Ignoring move intent due to distance from current position being too close"
                );
                return Err("Distance from current position too close".into());
            }
        }
        MoveIntentData::Path(path) => {
            if path.iter().any(|x| is_move_too_far(current, (*x).into())) {
                log::info!(
                    "Ignoring move intent due to distance from current position being too far"
                );
                return Err("Distance from current position too far".into());
            }
        }
        MoveIntentData::Actor(owner) => {
            let Some(target) = ctx.db.transform_tbl().actor_id().find(owner) else {
                log::error!("Unable to find target for move intent");
                return Err("Unable to find target for move intent".into());
            };

            // Only check if the actor is too far because this can be used to follow, even when close.
            if is_move_too_far(current, target.data.translation.xz().into()) {
                log::info!(
                    "Ignoring move intent due to distance from current position being too far"
                );
                return Err("Distance from current position too far".into());
            }
        }
    }

    movement_state.move_intent = Some(intent);
    movement_state.should_move = true;

    ctx.db
        .movement_state_tbl()
        .actor_id()
        .update(movement_state);

    Ok(())
}

#[reducer]
pub fn cancel_move(ctx: &ReducerContext) -> Result<(), String> {
    let Some(ci) = ctx.db.character_instance_tbl().identity().find(ctx.sender) else {
        return Err("Unable to find active character".into());
    };

    let Some(mut movement_state) = ctx.db.movement_state_tbl().actor_id().find(ci.actor_id) else {
        return Err("Unable to find movement state for the active character".into());
    };

    movement_state.move_intent = None;
    movement_state.should_move = !movement_state.grounded;

    ctx.db
        .movement_state_tbl()
        .actor_id()
        .update(movement_state);

    Ok(())
}
