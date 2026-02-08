use crate::{
    actor::{ActorEntityMapping, ensure_actor_entity},
    module_bindings::TransformRow,
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

#[derive(Component, Debug)]
pub struct NetTransform {
    pub translation: Vec3,
    pub client_intent_seq: u32,
}
#[derive(Component, Debug)]
pub struct SimTransform {
    pub translation: Vec3,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, (on_inserted, on_updated));
}

/// Handle the insertion of a new transform, this is the first point we have the
/// server's position so we directly set our position from it.
fn on_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<TransformRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);
        let translation: Vec3 = msg.row.translation.clone().into();
        commands.entity(bevy_entity).insert((
            Visibility::Inherited,
            Transform {
                translation,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            NetTransform {
                translation,
                client_intent_seq: msg.row.client_intent_seq,
            },
            SimTransform { translation },
        ));
    }
}

/// A listener during update of messages sent from the server for the transform data changing.
fn on_updated(
    mut transform_q: Query<&mut NetTransform>,
    mut msgs: ReadUpdateMessage<TransformRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut net_transform) = transform_q.get_mut(bevy_entity) else {
            continue;
        };
        net_transform.translation = msg.new.translation.clone().into();
    }
}
