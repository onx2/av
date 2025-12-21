use super::{LocalPlayer, NetworkTransform, Player, RemotePlayer};
use crate::{
    module_bindings::{Actor, AoiTransformDataTableAccess, TransformData},
    player::NetworkTransformEntityMapping,
    server::SpacetimeDB,
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadDeleteMessage, ReadInsertMessage, ReadUpdateMessage};
use spacetimedb_sdk::Table;

pub(super) fn on_actor_deleted(
    mut commands: Commands,
    mut msgs: ReadDeleteMessage<Actor>,
    mut entity_mapping: ResMut<NetworkTransformEntityMapping>,
) {
    for msg in msgs.read() {
        println!("REMOVED: {:?}", msg.row);
        if let Some(bevy_entity) = entity_mapping.0.remove(&msg.row.transform_data_id) {
            commands.entity(bevy_entity).despawn();
        }
    }
}

pub(super) fn on_actor_inserted(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stdb: SpacetimeDB,
    mut msgs: ReadInsertMessage<Actor>,
    mut actor_entity_mapping: ResMut<NetworkTransformEntityMapping>,
) {
    for msg in msgs.read() {
        println!("INSERTED: {:?}", msg.row);
        let new_actor = msg.row.clone();
        let is_local = new_actor.identity == Some(stdb.identity());
        let base_color = if is_local {
            Color::linear_rgb(0.2, 0.9, 0.8)
        } else {
            Color::linear_rgb(0.9, 0.2, 0.2)
        };

        let Some(transform_data) = stdb
            .db()
            .aoi_transform_data()
            .iter()
            .find(|data| data.id == msg.row.transform_data_id)
        else {
            println!("Failed to find transform data for actor {:?}", new_actor);
            continue;
        };

        let translation: Vec3 = transform_data.translation.into();
        let rotation = Quat::from_rotation_y(transform_data.yaw);

        let mut entity_commands = commands.spawn((
            Mesh3d(meshes.add(Mesh::from(Capsule3d {
                radius: new_actor.capsule_radius,
                half_length: new_actor.capsule_half_height,
            }))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color,
                ..default()
            })),
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
            NetworkTransform {
                translation,
                rotation,
            },
            Player,
        ));

        // Eyes (keep simple / same for all)
        entity_commands.with_children(|parent| {
            let eye_mesh = meshes.add(Mesh::from(Sphere { radius: 0.12 }));
            let eye_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 1.0),
                ..default()
            });

            let x = 0.18;
            let y = new_actor.capsule_half_height;
            let z = -new_actor.capsule_radius;

            parent.spawn((
                Name::new("LeftEye"),
                Mesh3d(eye_mesh.clone()),
                MeshMaterial3d(eye_mat.clone()),
                Transform::from_translation(Vec3::new(-x, y, z)),
            ));
            parent.spawn((
                Name::new("RightEye"),
                Mesh3d(eye_mesh),
                MeshMaterial3d(eye_mat),
                Transform::from_translation(Vec3::new(x, y, z)),
            ));
        });

        // Only players get local/remote tags (monsters get neither).
        entity_commands.insert_if(LocalPlayer, || is_local);
        entity_commands.insert_if(RemotePlayer, || !is_local);

        let bevy_entity = entity_commands.id();
        actor_entity_mapping
            .0
            .insert(new_actor.transform_data_id, bevy_entity);
    }
}

pub(super) fn sync(
    mut actor_q: Query<&mut NetworkTransform, With<Player>>,
    mut messages: ReadUpdateMessage<TransformData>,
    net_transform_entity_mapping: Res<NetworkTransformEntityMapping>,
) {
    for msg in messages.read() {
        println!("UPDATED: {:?}", msg.new);
        let transform_data = msg.new.clone();
        // Pull the authoritative row from the local STDB cache.
        let Some(bevy_entity) = net_transform_entity_mapping.0.get(&transform_data.id) else {
            continue;
        };
        let Ok(mut network_transform) = actor_q.get_mut(*bevy_entity) else {
            continue;
        };

        network_transform.translation = transform_data.translation.into();
        network_transform.rotation = Quat::from_rotation_y(transform_data.yaw);
    }
}
