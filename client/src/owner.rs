use crate::{
    module_bindings::{ActiveCharacterRow, TransformRow},
    server::SpacetimeDB,
};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_spacetimedb::{ReadInsertMessage, ReadInsertUpdateMessage, ReadUpdateMessage};
use shared::Owner;

#[derive(Resource, Default)]
pub struct OwnerEntityMapping(pub HashMap<Owner, Entity>);

#[derive(Component, Debug)]
pub struct OwnerEntity(pub Owner);

#[derive(Component, Debug)]
pub struct LocalOwner;

#[derive(Component, Debug)]
pub struct RemoteOwner;

/// Cached server transform data for an entity.
#[derive(Component, Debug)]
pub struct NetTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(OwnerEntityMapping::default());
    app.add_systems(
        Update,
        (on_active_character_inserted, on_monster_instance_inserted),
    );

    app.add_systems(Update, (on_transform_inserted, on_transform_updated));
}

fn on_transform_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertUpdateMessage<TransformRow>,
    oe_mapping: Res<OwnerEntityMapping>,
) {
    for msg in msgs.read() {
        let transform_data = msg.new.clone();
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

fn on_active_character_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<ActiveCharacterRow>,
    mut oe_mapping: ResMut<OwnerEntityMapping>,
    stdb: SpacetimeDB,
) {
    for msg in msgs.read() {
        if oe_mapping.0.contains_key(&msg.row.owner) {
            continue;
        }

        let is_local = msg.row.identity == stdb.identity();
        let mut entity = commands.spawn((OwnerEntity(msg.row.owner),));
        entity.insert_if(LocalOwner, || is_local);
        entity.insert_if(RemoteOwner, || !is_local);
        oe_mapping.0.insert(msg.row.owner, entity.id());
    }
}

fn on_monster_instance_inserted(// mut commands: Commands,
    // mut msgs: ReadInsertMessage<Monster>,
    // mut oe_mapping: ResMut<OwnerEntityMapping>,
    // stdb: SpacetimeDB,
) {
    // for msg in msgs.read() {
    //     // Not sure when this would happen but probably shouldn't allow duplicates
    //     if oe_mapping.0.contains_key(&msg.row.owner) {
    //         continue;
    //     }

    //     let entity = if msg.row.identity == stdb.identity() {
    //         commands.spawn(LocalOwner(msg.row.owner))
    //     } else {
    //         commands.spawn(RemoteOwner(msg.row.owner))
    //     };
    //     oe_mapping.0.insert(msg.row.owner, entity.id());
    // }
}

// mut meshes: ResMut<Assets<Mesh>>,
// mut materials: ResMut<Assets<StandardMaterial>>,
// Transform::default(),
// Mesh3d(meshes.add(Mesh::from(Capsule3d {
//     radius: 0.3,
//     half_length: 0.85,
// }))),
// MeshMaterial3d(materials.add(StandardMaterial {
//     base_color,
//     ..default()
// })),

// fn on_character_deleted(mut commands: Commands) {
//     for msg in msgs.read() {
//         if let Some(bevy_entity) = actor_entity_mapping.0.remove(&msg.row.owner) {
//             commands.entity(bevy_entity).despawn();
//         }
//     }
// }
