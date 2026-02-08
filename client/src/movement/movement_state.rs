use crate::{
    ActorEntityMapping, ensure_actor_entity,
    module_bindings::{MoveIntentData, MovementStateRow},
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};
use shared::CellId;

#[derive(Component, Debug)]
pub struct NetMovementState {
    pub cell_id: CellId,
    pub should_move: bool,
    pub move_intent: MoveIntentData,
    pub vertical_velocity: i8,
    pub client_intent_seq: u32,
}

#[derive(Component, Debug)]
pub struct SimMovementState {
    pub cell_id: CellId,
    pub should_move: bool,
    pub move_intent: MoveIntentData,
    pub vertical_velocity: i8,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, (on_inserted, on_updated));
}

fn on_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<MovementStateRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_movement_state_inserted: {:?}", msg.row.actor_id);
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);
        commands.entity(bevy_entity).insert((
            NetMovementState {
                move_intent: msg.row.move_intent.clone(),
                cell_id: msg.row.cell_id,
                should_move: msg.row.should_move,
                vertical_velocity: msg.row.vertical_velocity,
                client_intent_seq: msg.row.client_intent_seq,
            },
            SimMovementState {
                move_intent: msg.row.move_intent.clone(),
                cell_id: msg.row.cell_id,
                should_move: msg.row.should_move,
                vertical_velocity: msg.row.vertical_velocity,
            },
        ));
    }
}

fn on_updated(
    mut net_movement_state_q: Query<&mut NetMovementState>,
    mut msgs: ReadUpdateMessage<MovementStateRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut net_movement_state) = net_movement_state_q.get_mut(bevy_entity) else {
            continue;
        };

        // println!("on_movement_state_updated: {:?}", msg.new.actor_id);
        net_movement_state.move_intent = msg.new.move_intent.clone();
        net_movement_state.cell_id = msg.new.cell_id;
        net_movement_state.should_move = msg.new.should_move;
        net_movement_state.vertical_velocity = msg.new.vertical_velocity;
    }
}
