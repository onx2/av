use crate::{
    cursor::{CurrentCursor, set_cursor_to_ability, set_cursor_to_combat, set_cursor_to_default},
    input::InputAction,
    module_bindings::{MoveIntent, enter_world, request_move, spawn_fake_remotes},
    server::SpacetimeDB,
};
use bevy::{picking::pointer::PointerInteraction, prelude::*};
use leafwing_input_manager::prelude::ActionState;
use shared::constants::{DIRECTIONAL_MOVEMENT_INTERVAL, SMALLEST_REQUEST_DISTANCE_SQ};
use std::time::Duration;

/// The time since the last directional movement was sent to the server.
#[derive(Resource, Default)]
pub(super) struct LastDirectionSentAt {
    pub duration: Duration,
    pub position: Vec3,
}

pub(super) fn handle_lmb_movement(
    mut last_sent_at: ResMut<LastDirectionSentAt>,
    actions: Res<ActionState<InputAction>>,
    interactions: Query<&PointerInteraction>,
    stdb: SpacetimeDB,
) {
    let just_pressed = actions.just_pressed(&InputAction::LeftClick);
    let pressed = actions.pressed(&InputAction::LeftClick);
    let just_released = actions.just_released(&InputAction::LeftClick);
    if !just_pressed && !pressed && !just_released {
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

    if just_pressed {
        if let Err(e) = stdb.reducers().request_move(MoveIntent::Point(pos.into())) {
            println!("Error: {e}");
        }
        return;
    }

    if just_released {
        // Reset the "Direct Movement" tracker so the next click feels fresh.
        last_sent_at.duration = Duration::ZERO;
        last_sent_at.position = Vec3::ZERO;

        match stdb.reducers().request_move(MoveIntent::Point(pos.into())) {
            Ok(_) => println!("JUST RELEASED: {:?}", pos),
            Err(e) => println!("Error: {e}"),
        }
        return;
    }

    // pressed (held)
    let distance_ready = last_sent_at.position.distance_squared(pos) > SMALLEST_REQUEST_DISTANCE_SQ;
    if !distance_ready {
        return;
    }

    let held_dur = actions.current_duration(&InputAction::LeftClick);
    if held_dur < Duration::from_millis(150) {
        return;
    }

    let timer_ready = held_dur == Duration::ZERO
        || held_dur.saturating_sub(last_sent_at.duration) >= DIRECTIONAL_MOVEMENT_INTERVAL;
    if !timer_ready {
        return;
    }

    match stdb.reducers().request_move(MoveIntent::Point(pos.into())) {
        Ok(_) => {
            last_sent_at.position = pos;
            last_sent_at.duration = held_dur;
        }
        Err(e) => println!("Error: {e}"),
    }
}

pub(super) fn handle_enter_world(
    current_cursor: ResMut<CurrentCursor>,
    keys: Res<ButtonInput<KeyCode>>,
    stdb: SpacetimeDB,
) {
    if keys.just_pressed(KeyCode::Space) {
        match stdb.reducers().enter_world() {
            Ok(_) => println!("Called enter world without immediate failure"),
            Err(err) => println!("Immediate failure when calling enter world: {err}"),
        }
    } else if keys.just_pressed(KeyCode::Digit1) {
        set_cursor_to_default(current_cursor);
    } else if keys.just_pressed(KeyCode::Digit2) {
        set_cursor_to_ability(current_cursor);
    } else if keys.just_pressed(KeyCode::Digit3) {
        set_cursor_to_combat(current_cursor);
    } else if keys.just_pressed(KeyCode::Digit0) {
        match stdb.reducers().spawn_fake_remotes(10) {
            Ok(_) => println!("Success: called spawn fake monsters"),
            Err(e) => eprintln!("{e:?}"),
        }
    }
}
