use crate::{
    ActorEntityMapping, ensure_actor_entity,
    module_bindings::{MoveIntentData, MovementStateRow},
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

#[derive(Component, Debug)]
pub struct MovementState {
    pub cell_id: u32,
    pub should_move: bool,
    pub move_intent: Option<MoveIntentData>,
    pub grounded: bool,
    pub vertical_velocity: f32,
    // pub capsule: Capsule, // TODO: predict collision on client
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PreUpdate,
        (on_movement_state_inserted, on_movement_state_updated),
    );
}

fn on_movement_state_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<MovementStateRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_movement_state_inserted: {:?}", msg.row.actor_id);
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);
        commands.entity(bevy_entity).insert(MovementState {
            move_intent: msg.row.move_intent.clone(),
            cell_id: msg.row.cell_id,
            should_move: msg.row.should_move,
            grounded: msg.row.grounded,
            vertical_velocity: msg.row.vertical_velocity,
        });
    }
}

fn on_movement_state_updated(
    mut movement_state_q: Query<&mut MovementState>,
    mut msgs: ReadUpdateMessage<MovementStateRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut movement_state) = movement_state_q.get_mut(bevy_entity) else {
            continue;
        };

        // println!("on_movement_state_updated: {:?}", msg.new.actor_id);
        movement_state.move_intent = msg.new.move_intent.clone();
        movement_state.cell_id = msg.new.cell_id;
        movement_state.should_move = msg.new.should_move;
        movement_state.grounded = msg.new.grounded;
        movement_state.vertical_velocity = msg.new.vertical_velocity;
    }
}
