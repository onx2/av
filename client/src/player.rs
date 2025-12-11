use crate::{
    module_bindings::{Actor, ActorKind, ActorTableAccess, MoveIntent, enter_world, request_move},
    server::SpacetimeDB,
};
use bevy::{picking::pointer::PointerInteraction, platform::collections::HashMap, prelude::*};
use bevy_spacetimedb::{ReadDeleteMessage, ReadInsertMessage, ReadUpdateMessage};

/// Used to tie the server entity id to the local bevy entity
#[derive(Resource, Default)]
pub struct ActorEntityMapping(HashMap<u64, Entity>);

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(ActorEntityMapping::default());
    app.add_systems(PreUpdate, sync);
    app.add_systems(Update, (handle_enter_world, handle_left_click));
    app.add_systems(
        PostUpdate,
        (on_actor_inserted, on_actor_deleted, draw_player_facing),
    );
}

#[derive(Component)]
pub struct Player {
    pub actor_id: u64,
}

// TODO are these necessary? seems like a waste of component when we have the identity and the stdb identity
#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component)]
pub struct RemotePlayer;

fn on_actor_deleted(
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

fn on_actor_inserted(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    stdb: SpacetimeDB,
    mut msgs: ReadInsertMessage<Actor>,
    mut actor_entity_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        if let Some(identity) = match msg.row.kind {
            ActorKind::Player(identity) => Some(identity),
            _ => None,
        } {
            let new_actor = msg.row.clone();
            let is_local = identity == stdb.identity();
            let base_color = match is_local {
                true => Color::linear_rgb(0.2, 0.9, 0.8),
                false => Color::linear_rgb(0.9, 0.2, 0.2),
            };

            let actor_id = msg.row.id;
            let translation = new_actor.translation.into();
            let rotation = new_actor.rotation.into();
            let scale = new_actor.scale.into();
            let bevy_entity = commands
                .spawn((
                    Mesh3d(meshes.add(Mesh::from(Capsule3d {
                        radius: 0.5,
                        half_length: 1.0,
                    }))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color,
                        ..default()
                    })),
                    Transform {
                        translation,
                        rotation,
                        scale,
                    },
                    Player { actor_id },
                ))
                .with_children(|parent| {
                    // Eyes: small white spheres, slightly in front (-Z is forward)
                    let eye_mesh = meshes.add(Mesh::from(Sphere { radius: 0.12 }));
                    let eye_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 1.0, 1.0),
                        ..default()
                    });

                    parent.spawn((
                        Name::new("LeftEye"),
                        Mesh3d(eye_mesh.clone()),
                        MeshMaterial3d(eye_mat.clone()),
                        Transform {
                            translation: Vec3::new(-0.18, 1.0, -0.55), // offset from center; slightly in front (-Z)
                            ..default()
                        },
                    ));

                    parent.spawn((
                        Name::new("RightEye"),
                        Mesh3d(eye_mesh),
                        MeshMaterial3d(eye_mat),
                        Transform {
                            translation: Vec3::new(0.18, 1.0, -0.55),
                            ..default()
                        },
                    ));
                })
                .insert_if(LocalPlayer, || is_local)
                .insert_if(RemotePlayer, || !is_local)
                .id();

            actor_entity_mapping.0.insert(msg.row.id, bevy_entity);
        }
    }
}

fn draw_player_facing(mut gizmos: Gizmos, q: Query<&GlobalTransform, With<Player>>) {
    for gt in &q {
        // Get world-space rotation/position robustly
        let (_, rot, start) = gt.to_scale_rotation_translation();

        // Compute forward from rotation; fallback if invalid/degenerate
        let mut dir = rot * Vec3::NEG_Z;
        if !dir.is_finite() || dir.length_squared() <= 1.0e-8 {
            // Use default forward to avoid zero-length direction panics
            dir = Vec3::NEG_Z;
        }

        // Draw arrow using start/end so we don't construct a Dir3 directly
        let end = start + dir.normalize() * 1.5;
        gizmos.arrow(start, end, Color::srgb(1.0, 1.0, 0.2));
    }
}

fn handle_left_click(
    mb: Res<ButtonInput<MouseButton>>,
    interactions: Query<&PointerInteraction, Without<LocalPlayer>>,
    stdb: SpacetimeDB,
) {
    if !mb.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(interaction) = interactions.single() else {
        return;
    };
    let Some((_entity, hit)) = interaction.get_nearest_hit() else {
        return;
    };
    let Some(pos) = hit.position else {
        return;
    };

    if let Ok(_) = stdb.reducers().request_move(MoveIntent::Point(pos.into())) {
        println!("Sent move request: {:?}", pos);
    } else {
        println!("Failed to request move");
        return;
    };
}

fn handle_enter_world(keys: Res<ButtonInput<KeyCode>>, stdb: SpacetimeDB) {
    if keys.just_pressed(KeyCode::Space) {
        match stdb.reducers().enter_world() {
            Ok(_) => {
                println!("Called enter world without immediate failure");
            }
            Err(err) => {
                println!("Immediate failure when calling enter world: {}", err);
            }
        }
    }
}

fn sync(
    mut transform_query: Query<&mut Transform, With<Player>>,
    stdb: SpacetimeDB,
    mut messages: ReadUpdateMessage<Actor>,
    actor_entity_mapping: Res<ActorEntityMapping>,
) {
    for msg in messages.read() {
        if let Some(actor) = stdb.db().actor().id().find(&msg.new.id) {
            if let Some(bevy_entity) = actor_entity_mapping.0.get(&actor.id) {
                if let Ok((mut transform)) = transform_query.get_mut(*bevy_entity) {
                    transform.rotation = actor.rotation.into();
                    transform.scale = actor.scale.into();
                    transform.translation = actor.translation.into();
                }
            }
        }
    }
}
