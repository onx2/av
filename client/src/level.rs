use crate::{ActorEntityMapping, ensure_actor_entity, module_bindings::LevelRow};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

#[derive(Component, Debug)]
pub struct Level(pub u8);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, (on_level_inserted, on_level_updated));
}

fn on_level_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<LevelRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_level_inserted: {:?}", msg.row.clone());
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);
        commands.entity(bevy_entity).insert(Level(msg.row.level));
    }
}

fn on_level_updated(
    mut level_q: Query<&mut Level>,
    mut msgs: ReadUpdateMessage<LevelRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut level) = level_q.get_mut(bevy_entity) else {
            continue;
        };
        level.0 = msg.new.level;
    }
}
