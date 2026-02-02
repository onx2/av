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
            // Make visible now that we have a valid transform. TODO: this might not be necessary once assets for the character are used.
            Visibility::Inherited,
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
    mut transform_q: Query<&mut NetTransform>,
    mut msgs: ReadUpdateMessage<TransformRow>,
    oe_mapping: Res<OwnerEntityMapping>,
) {
    for msg in msgs.read() {
        let transform = msg.new.clone();
        let Some(&bevy_entity) = oe_mapping.0.get(&transform.owner) else {
            continue;
        };
        let Ok(mut net_transform) = transform_q.get_mut(bevy_entity) else {
            continue;
        };
        // println!("on_transform_updated: {:?}", transform.owner);
        net_transform.translation = transform.data.translation.into();
        net_transform.rotation = transform.data.rotation.into();
    }
}

fn interpolate(time: Res<Time>, mut transform_q: Query<(&mut Transform, &NetTransform)>) {
    let dt = time.delta_secs();
    transform_q.par_iter_mut().for_each(|(mut transform, net)| {
        transform
            .translation
            .smooth_nudge(&net.translation, 12.0, dt);
        transform.rotation = transform
            .rotation
            .slerp(net.rotation, 1.0 - (-24.0 * dt).exp());
    });
}
