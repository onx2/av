use crate::{
    module_bindings::TransformRow,
    owner::{OwnerEntityMapping, ensure_owner_entity},
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
    app.add_systems(PreUpdate, (on_transform_inserted, on_transform_updated));
    app.add_systems(PostUpdate, interpolate);
}

fn on_transform_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<TransformRow>,
    mut oe_mapping: ResMut<OwnerEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_transform_inserted: {:?}", msg.row.owner);
        let transform_data = msg.row.clone();

        // Ensure the owner entity exists regardless of message ordering.
        let bevy_entity = ensure_owner_entity(&mut commands, &mut oe_mapping, transform_data.owner);

        // Use Commands to avoid timing issues with deferred spawns/components.
        let translation: Vec3 = transform_data.data.translation.into();
        let rotation: Quat = transform_data.data.rotation.into();

        commands.entity(bevy_entity).insert((
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
            NetTransform {
                translation,
                rotation,
            },
        ));
    }
}

fn on_transform_updated(
    mut commands: Commands,
    mut msgs: ReadUpdateMessage<TransformRow>,
    mut oe_mapping: ResMut<OwnerEntityMapping>,
) {
    for msg in msgs.read() {
        let transform_data = msg.new.clone();
        let bevy_entity = ensure_owner_entity(&mut commands, &mut oe_mapping, transform_data.owner);

        commands.entity(bevy_entity).insert(NetTransform {
            translation: transform_data.data.translation.into(),
            rotation: transform_data.data.rotation.into(),
        });
    }
}

fn interpolate(time: Res<Time>, mut transform_q: Query<(&mut Transform, &NetTransform)>) {
    let dt = time.delta_secs();
    transform_q.par_iter_mut().for_each(|(mut transform, net)| {
        transform
            .translation
            .smooth_nudge(&net.translation, 18.0, dt);
        transform.rotation = transform
            .rotation
            .slerp(net.rotation, 1.0 - (-24.0 * dt).exp());
    });
}
