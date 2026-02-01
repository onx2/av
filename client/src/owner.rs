use crate::{module_bindings::ActiveCharacterRow, server::SpacetimeDB};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_spacetimedb::ReadInsertMessage;
use shared::Owner;

#[derive(Resource, Default)]
pub struct OwnerEntityMapping(pub HashMap<Owner, Entity>);

#[derive(Component, Debug)]
pub struct OwnerEntity(pub Owner);

#[derive(Component, Debug)]
pub struct LocalOwner;

#[derive(Component, Debug)]
pub struct RemoteOwner;

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(OwnerEntityMapping::default());
    app.add_systems(
        Update,
        (on_active_character_inserted, on_monster_instance_inserted),
    );
}

fn on_active_character_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<ActiveCharacterRow>,
    mut oe_mapping: ResMut<OwnerEntityMapping>,
    stdb: SpacetimeDB,
) {
    for msg in msgs.read() {
        println!("on_active_character_inserted");
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
