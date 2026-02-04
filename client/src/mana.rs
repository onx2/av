use crate::{ActorEntityMapping, ensure_actor_entity, module_bindings::ManaRow};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

#[derive(Component, Debug)]
pub struct Mana {
    pub current: u16,
    pub max: u16,
    pub is_full: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, (on_mana_inserted, on_mana_updated));
}

fn on_mana_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<ManaRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_mana_inserted: {:?}", msg.row.clone());
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);
        commands.entity(bevy_entity).insert(Mana {
            current: msg.row.data.current,
            max: msg.row.data.max,
            is_full: msg.row.is_full,
        });
    }
}

fn on_mana_updated(
    mut manaq: Query<&mut Mana>,
    mut msgs: ReadUpdateMessage<ManaRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut mana) = manaq.get_mut(bevy_entity) else {
            continue;
        };
        mana.current = msg.new.data.current;
        mana.max = msg.new.data.max;
        mana.is_full = msg.new.is_full;
    }
}
