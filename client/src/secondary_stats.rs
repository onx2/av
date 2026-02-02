use crate::{OwnerEntityMapping, ensure_owner_entity, module_bindings::SecondaryStatsRow};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

#[derive(Component, Debug)]
pub struct SecondaryStats {
    pub movement_speed: f32,
    pub critical_hit_chance: f32,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PreUpdate,
        (on_secondary_stats_inserted, on_secondary_stats_updated),
    );
}

fn on_secondary_stats_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<SecondaryStatsRow>,
    mut oe_mapping: ResMut<OwnerEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_secondary_stats_inserted: {:?}", msg.row.owner);
        let bevy_entity = ensure_owner_entity(&mut commands, &mut oe_mapping, msg.row.owner);
        commands.entity(bevy_entity).insert(SecondaryStats {
            movement_speed: msg.row.data.movement_speed,
            critical_hit_chance: msg.row.data.critical_hit_chance,
        });
    }
}

fn on_secondary_stats_updated(
    mut secondary_stats_q: Query<&mut SecondaryStats>,
    mut msgs: ReadUpdateMessage<SecondaryStatsRow>,
    oe_mapping: Res<OwnerEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.owner) else {
            continue;
        };
        let Ok(mut secondary_stats) = secondary_stats_q.get_mut(bevy_entity) else {
            continue;
        };
        secondary_stats.movement_speed = msg.new.data.movement_speed;
        secondary_stats.critical_hit_chance = msg.new.data.critical_hit_chance;
    }
}
