use crate::{
    module_bindings::TransformRow,
    owner::{OwnerEntity, OwnerEntityMapping},
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};

/// Cached server transform data for an entity.
#[derive(Component, Debug)]
pub struct NetTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (on_transform_inserted, on_transform_updated));
}

fn on_transform_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<TransformRow>,
    oe_mapping: Res<OwnerEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_transform_inserted");
        let transform_data = msg.row.clone();
        let Some(bevy_entity) = oe_mapping.0.get(&transform_data.owner) else {
            continue;
        };

        let transform_bundle = Transform {
            translation: transform_data.data.translation.into(),
            rotation: transform_data.data.rotation.into(),
            scale: Vec3::ONE,
        };

        commands.entity(*bevy_entity).insert((
            transform_bundle,
            NetTransform {
                translation: transform_bundle.translation,
                rotation: transform_bundle.rotation,
            },
        ));
    }
}

fn on_transform_updated(
    mut owner_query: Query<&mut NetTransform, With<OwnerEntity>>,
    mut msgs: ReadUpdateMessage<TransformRow>,
    oe_mapping: Res<OwnerEntityMapping>,
) {
    // TODO: This should either update a custom component then we can interpolate between the old and new values.
    for msg in msgs.read() {
        println!("on_transform_updated");
        let transform_data = msg.new.clone();
        let Some(bevy_entity) = oe_mapping.0.get(&transform_data.owner) else {
            continue;
        };
        let Ok(mut net_transform) = owner_query.get_mut(*bevy_entity) else {
            continue;
        };
        net_transform.translation = transform_data.data.translation.into();
        net_transform.rotation = transform_data.data.rotation.into();
    }
}
