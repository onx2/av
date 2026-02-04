use crate::{ActorEntityMapping, ensure_actor_entity, module_bindings::ExperienceRow};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

#[derive(Component, Debug)]
pub struct Experience(pub u32);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, (on_experience_inserted, on_experience_updated));
}

fn on_experience_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<ExperienceRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_experience_inserted: {:?}", msg.row.clone());
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);
        commands.entity(bevy_entity).insert(Experience(msg.row.xp));
    }
}

fn on_experience_updated(
    mut experienceq: Query<&mut Experience>,
    mut msgs: ReadUpdateMessage<ExperienceRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut experience) = experienceq.get_mut(bevy_entity) else {
            continue;
        };
        experience.0 = msg.new.xp;
    }
}
