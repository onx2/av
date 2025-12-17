use super::{ActorEntityMapping, LocalPlayer, NetworkActor, Player, RemotePlayer};
use crate::{
    module_bindings::{Actor, ActorKind, ActorTableAccess},
    server::SpacetimeDB,
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadDeleteMessage, ReadInsertMessage, ReadUpdateMessage};

fn is_monster(kind: &ActorKind) -> bool {
    matches!(kind, ActorKind::Monster(_))
}
fn is_local_player(kind: &ActorKind, stdb: &SpacetimeDB) -> bool {
    matches!(kind, ActorKind::Player(identity) if *identity == stdb.identity())
}
fn is_remote_player(kind: &ActorKind, stdb: &SpacetimeDB) -> bool {
    matches!(kind, ActorKind::Player(identity) if *identity != stdb.identity())
}
fn actor_color(kind: &ActorKind, stdb: &SpacetimeDB) -> Color {
    if is_monster(kind) {
        Color::linear_rgb(0.8, 0.6, 0.1)
    } else if is_local_player(kind, stdb) {
        Color::linear_rgb(0.2, 0.9, 0.8)
    } else {
        // Remote player
        Color::linear_rgb(0.9, 0.2, 0.2)
    }
}

pub(super) fn on_actor_deleted(
    mut commands: Commands,
    mut msgs: ReadDeleteMessage<Actor>,
    mut entity_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        if let Some(bevy_entity) = entity_mapping.0.remove(&msg.row.id) {
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
    mut actor_entity_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let new_actor = msg.row.clone();

        let base_color = actor_color(&new_actor.kind, &stdb);

        let actor_id = new_actor.id;
        let move_intent = new_actor.move_intent;
        let translation: Vec3 = new_actor.translation.into();
        let rotation = Quat::from_rotation_y(new_actor.yaw);

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
            NetworkActor {
                actor_id,
                translation,
                rotation,
                move_intent,
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
        entity_commands.insert_if(LocalPlayer, || is_local_player(&new_actor.kind, &stdb));
        entity_commands.insert_if(RemotePlayer, || is_remote_player(&new_actor.kind, &stdb));

        let bevy_entity = entity_commands.id();
        actor_entity_mapping.0.insert(actor_id, bevy_entity);
    }
}

pub(super) fn sync(
    mut actor_q: Query<&mut NetworkActor, With<Player>>,
    mut messages: ReadUpdateMessage<Actor>,
    stdb: SpacetimeDB,
    actor_entity_mapping: Res<ActorEntityMapping>,
) {
    for msg in messages.read() {
        // Pull the authoritative row from the local STDB cache.
        let Some(actor) = stdb.db().actor().id().find(&msg.new.id) else {
            continue;
        };
        let Some(bevy_entity) = actor_entity_mapping.0.get(&actor.id) else {
            continue;
        };
        let Ok(mut network_actor) = actor_q.get_mut(*bevy_entity) else {
            continue;
        };

        network_actor.translation = actor.translation.into();
        network_actor.rotation = Quat::from_rotation_y(actor.yaw);
        network_actor.move_intent = actor.move_intent;
        network_actor.actor_id = actor.id;
    }
}
