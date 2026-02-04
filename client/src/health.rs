use crate::{ActorEntityMapping, ensure_actor_entity, module_bindings::HealthRow};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

#[derive(Component, Debug)]
pub struct Health {
    pub current: u16,
    pub max: u16,
    pub is_full: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, (on_health_inserted, on_health_updated));
}

fn on_health_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<HealthRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_health_inserted: {:?}", msg.row.clone());
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);
        commands.entity(bevy_entity).insert(Health {
            current: msg.row.data.current,
            max: msg.row.data.max,
            is_full: msg.row.is_full,
        });
    }
}

fn on_health_updated(
    mut healthq: Query<&mut Health>,
    mut msgs: ReadUpdateMessage<HealthRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut health) = healthq.get_mut(bevy_entity) else {
            continue;
        };
        health.current = msg.new.data.current;
        health.max = msg.new.data.max;
        health.is_full = msg.new.is_full;
    }
}
