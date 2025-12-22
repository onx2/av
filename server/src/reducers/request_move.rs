use crate::{schema::*, types::MoveIntent};
use nalgebra as na;
use shared::{
    constants::{MAX_INTENT_PATH_LEN, Y_QUANTIZE_STEP_M},
    utils::{is_move_too_close, is_move_too_far},
};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn request_move(ctx: &ReducerContext, intent: MoveIntent) -> Result<(), String> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return Err("Player not found".into());
    };
    let Some(source_actor_id) = player.actor_id else {
        return Err("Actor not found".into());
    };
    let Some(source_actor) = ctx.db.actor().id().find(source_actor_id) else {
        return Err("Actor not found".into());
    };
    let Some(transform_data) = ctx
        .db
        .transform_data()
        .id()
        .find(source_actor.transform_data_id)
    else {
        return Err("Transform data not found".into());
    };
    let Some(mut movement_data) = ctx
        .db
        .movement_data()
        .id()
        .find(source_actor.movement_data_id)
    else {
        return Err("Transform data not found".into());
    };

    // `TransformData.translation` is mixed precision (`DbVec3i16`):
    // - x/z are already meters (f32)
    // - y is quantized (i16, `Y_QUANTIZE_STEP_M` units)
    //
    // Decode to meters for intent validation math.
    let current: na::Vector3<f32> = na::Vector3::new(
        transform_data.translation.x,
        transform_data.translation.y as f32 * Y_QUANTIZE_STEP_M,
        transform_data.translation.z,
    );
    match (&movement_data.move_intent, &intent) {
        // 1. Idling Check
        (MoveIntent::None, MoveIntent::None) => {
            return Err("Already idling".into());
        }

        // 2. History Check: Is the new point too close to the old intent point?
        (MoveIntent::Point(old), MoveIntent::Point(new))
            if is_move_too_close(&old.into(), &new.into()) =>
        {
            return Err("Distance from last point too close".into());
        }

        // 3. Path Validation: Complexity check
        (_, MoveIntent::Path(p)) if p.len() > MAX_INTENT_PATH_LEN => {
            return Err("Path is too complex".into());
        }

        // 4. Path Validation: Range check (are any points too far?)
        (_, MoveIntent::Path(p)) if p.iter().any(|x| is_move_too_far(&current, &x.into())) => {
            return Err("Distance from current position too far".into());
        }

        // 5. Point Validation: Minimum movement check (from current position)
        (_, MoveIntent::Point(p)) if is_move_too_close(&current, &p.into()) => {
            return Err("Distance from current position too close".into());
        }

        _ => {
            // Compute before updating to avoid using `movement_data` after it's moved into `update(...)`.
            let should_move = intent != MoveIntent::None || !movement_data.grounded;

            movement_data.should_move = should_move;
            movement_data.move_intent = intent;
            ctx.db.movement_data().id().update(movement_data);

            // Keep the duplicated flag on `Actor` consistent with `MovementData.should_move`.
            // Movement ticks iterate `actor(should_move, is_player)`, so if this isn't set,
            // the actor will never be processed.
            if source_actor.should_move != should_move {
                ctx.db.actor().id().update(Actor {
                    should_move,
                    ..source_actor
                });
            }

            Ok(())
        }
    }
}
