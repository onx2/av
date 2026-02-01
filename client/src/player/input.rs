use crate::{
    // actor::{LocalActor, MovementData},
    cursor::{CurrentCursor, set_cursor_to_ability, set_cursor_to_combat, set_cursor_to_default},
    input::InputAction,
    module_bindings::{MoveIntentData, create_character, enter_game, request_move},
    owner::LocalOwner,
    server::SpacetimeDB,
};
use bevy::{picking::pointer::PointerInteraction, prelude::*};
use leafwing_input_manager::prelude::ActionState;

pub(super) fn handle_lmb_movement(
    // mut local_actor_q: Single<&mut MovementData, With<LocalOwner>>,
    actions: Res<ActionState<InputAction>>,
    interactions: Query<&PointerInteraction>,
    stdb: SpacetimeDB,
) {
    let pressed = actions.pressed(&InputAction::LeftClick);
    let just_released = actions.just_released(&InputAction::LeftClick);
    if !pressed && !just_released {
        return;
    }
    let Ok(interaction) = interactions.single() else {
        return;
    };
    let Some((_entity, hit)) = interaction.get_nearest_hit() else {
        return;
    };
    let Some(pos) = hit.position else {
        return;
    };

    // TODO: just_released should request path move, for now everything is point
    if pressed || just_released {
        match stdb
            .reducers()
            .request_move(MoveIntentData::Point(crate::module_bindings::Vec2 {
                x: pos.x,
                z: pos.z,
            })) {
            Ok(_) => {
                // local_actor_q.move_intent = MoveIntentData::Point(pos.into());
            }
            Err(e) => println!("Error: {e}"),
        }
        return;
    }
}

pub(super) fn handle_enter_world(
    current_cursor: ResMut<CurrentCursor>,
    keys: Res<ButtonInput<KeyCode>>,
    stdb: SpacetimeDB,
) {
    if keys.just_pressed(KeyCode::Space) {
        let foo = stdb.reducers().create_character("Jeff".into());
        match stdb.reducers().enter_game(1) {
            Ok(_) => println!("Called enter world without immediate failure"),
            Err(err) => println!("Immediate failure when calling enter world: {err}"),
        }
    } else if keys.just_pressed(KeyCode::Digit1) {
        set_cursor_to_default(current_cursor);
    } else if keys.just_pressed(KeyCode::Digit2) {
        set_cursor_to_ability(current_cursor);
    } else if keys.just_pressed(KeyCode::Digit3) {
        set_cursor_to_combat(current_cursor);
    }
}
