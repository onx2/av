use super::{NetworkTransform, NetworkTransformDataEntityMapping};
use crate::{
    actor::{LocalActor, MovementData, RemoteActor},
    module_bindings::{
        AoiActor, AoiSecondaryStatsTableAccess, AoiTransformDataTableAccess, TransformData,
    },
    server::SpacetimeDB,
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadDeleteMessage, ReadInsertMessage, ReadUpdateMessage};
use shared::utils::yaw_from_u16;
use spacetimedb_sdk::Table;

pub(super) fn on_actor_deleted(
    mut commands: Commands,
    mut msgs: ReadDeleteMessage<AoiActor>,
    mut actor_entity_mapping: ResMut<NetworkTransformDataEntityMapping>,
) {
    for msg in msgs.read() {
        if let Some(bevy_entity) = actor_entity_mapping.0.remove(&msg.row.transform_data_id) {
            commands.entity(bevy_entity).despawn();
        }
    }
}

pub(super) fn on_actor_inserted(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stdb: SpacetimeDB,
    mut msgs: ReadInsertMessage<AoiActor>,
    mut net_mapping: ResMut<NetworkTransformDataEntityMapping>,
) {
    for msg in msgs.read() {
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
            .find(|data| data.id == new_actor.transform_data_id)
        else {
            println!("Failed to find transform data for actor {:?}", new_actor);
            continue;
        };

        let Some(secondary_stats) = stdb
            .db()
            .aoi_secondary_stats()
            .iter()
            .find(|data| data.id == new_actor.secondary_stats_id)
        else {
            println!("Failed to find secondary stats for actor {:?}", new_actor);
            continue;
        };

        let translation: Vec3 = transform_data.translation.into();
        let rotation = Quat::from_rotation_y(yaw_from_u16(transform_data.yaw));

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
            MovementData {
                move_intent: new_actor.move_intent,
                grounded: new_actor.grounded,
                movement_speed: secondary_stats.movement_speed,
            },
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
        entity_commands.insert_if(LocalActor, || is_local);
        entity_commands.insert_if(RemoteActor, || !is_local);

        let bevy_entity = entity_commands.id();
        net_mapping
            .0
            .insert(new_actor.transform_data_id, bevy_entity);
    }
}

pub(super) fn sync_transform(
    mut actor_q: Query<&mut NetworkTransform>,
    mut messages: ReadUpdateMessage<TransformData>,
    net_mapping: Res<NetworkTransformDataEntityMapping>,
) {
    for msg in messages.read() {
        let transform_data = msg.new.clone();
        let Some(bevy_entity) = net_mapping.0.get(&transform_data.id) else {
            continue;
        };
        let Ok(mut network_transform) = actor_q.get_mut(*bevy_entity) else {
            continue;
        };

        println!(
            "Syncing transform for entity {:?}",
            transform_data.translation
        );
        network_transform.translation = transform_data.translation.into();
        network_transform.rotation = Quat::from_rotation_y(yaw_from_u16(transform_data.yaw));
    }
}

pub(super) fn sync_aoi_actor(
    mut actor_q: Query<&mut MovementData>,
    mut messages: ReadUpdateMessage<AoiActor>,
    net_mapping: Res<NetworkTransformDataEntityMapping>,
) {
    for msg in messages.read() {
        let aoi_actor = msg.new.clone();
        let Some(bevy_entity) = net_mapping.0.get(&aoi_actor.transform_data_id) else {
            continue;
        };
        let Ok(mut actor_move_intent) = actor_q.get_mut(*bevy_entity) else {
            continue;
        };
        actor_move_intent.move_intent = aoi_actor.move_intent;
        actor_move_intent.grounded = aoi_actor.grounded;
    }
}

// pub(super) fn sync_aoi_secondary_data(
//     mut actor_q: Query<&mut MovementData>,
//     mut messages: ReadUpdateMessage<AoiSecondaryData>,
//     net_mapping: Res<NetworkTransformDataEntityMapping>,
// ) {
//     for msg in messages.read() {
//         let aoi_secondary_data = msg.new.clone();
//         let Some(bevy_entity) = net_mapping.0.get(&aoi_secondary_data.transform_data_id) else {
//             continue;
//         };
//         let Ok(mut actor_move_intent) = actor_q.get_mut(*bevy_entity) else {
//             continue;
//         };
//         actor_move_intent.move_intent = aoi_actor.move_intent;
//         actor_move_intent.grounded = aoi_actor.grounded;
//     }
// }

// TODO sync_secondary_stats... need this for real-time updates of the movement_speed in MovementData
