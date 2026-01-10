mod input;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    // Gameplay/input systems
    app.add_systems(
        Update,
        (input::handle_enter_world, input::handle_lmb_movement),
    );
}
