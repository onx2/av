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
    let Some(mut source_actor) = ctx.db.actor().id().find(source_actor_id) else {
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

    match (&source_actor.move_intent, &intent) {
        // 1. Idling Check
        (MoveIntent::Idle(_), MoveIntent::Idle(_)) => {
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

        // 6. Otherwise, accept and write intent directly onto Actor.
        _ => {
            source_actor.move_intent = intent;

            // Keep should_move consistent:
            // - should_move if we have a non-idle intent, OR if we're airborne (gravity needs processing)
            let is_idle = matches!(source_actor.move_intent, MoveIntent::Idle(_));
            source_actor.should_move = !is_idle || !source_actor.grounded;

            ctx.db.actor().id().update(source_actor);
            Ok(())
        }
    }
}
