use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

#[derive(Reflect, Actionlike, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputAction {
    LeftClick,
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(InputManagerPlugin::<InputAction>::default());

    app.register_type::<InputAction>();

    let mut input_map = InputMap::<InputAction>::default();
    input_map.insert(InputAction::LeftClick, MouseButton::Left);
    app.insert_resource(input_map);
    app.insert_resource(ActionState::<InputAction>::default());
}
