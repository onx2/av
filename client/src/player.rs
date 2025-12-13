use std::time::Duration;

use crate::{
    cursor::{CurrentCursor, set_cursor_to_ability, set_cursor_to_combat, set_cursor_to_default},
    input::InputAction,
    module_bindings::{Actor, ActorKind, ActorTableAccess, MoveIntent, enter_world, request_move},
    server::SpacetimeDB,
    world::Ground,
};
use bevy::{picking::pointer::PointerInteraction, platform::collections::HashMap, prelude::*};
use bevy_spacetimedb::{ReadDeleteMessage, ReadInsertMessage, ReadUpdateMessage};
use leafwing_input_manager::prelude::ActionState;

/// How frequently, in milliseconds, to send directional movement updates to the server.
const DIRECTIONAL_MOVEMENT_INTERVAL: Duration = Duration::from_millis(50);

/// The time since the last directional movement was sent to the server.
#[derive(Resource, Default)]
struct LastDirectionSentAt {
    duration: Duration,
    position: Vec3,
}

/// Used to tie the server entity id to the local bevy entity
#[derive(Resource, Default)]
pub struct ActorEntityMapping(pub HashMap<u64, Entity>);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LastDirectionSentAt>();
    app.insert_resource(ActorEntityMapping::default());
    app.add_systems(PreUpdate, sync);
    app.add_systems(
        Update,
        (
            handle_enter_world,
            handle_lmb_combat,
            handle_lmb_movement,
            handle_lmb_ability,
        ),
    );
    app.add_systems(
        PostUpdate,
        (
            on_actor_inserted,
            on_actor_deleted,
            draw_player_facing,
            interpolate,
        ),
    );
}

#[derive(Component)]
pub struct NetworkTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
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
                        scale,
                    },
                    NetworkTransform {
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
                            translation: Vec3::new(
                                -0.18,
                                new_actor.capsule_half_height,
                                -new_actor.capsule_radius,
                            ), // offset from center; slightly in front (-Z)
                            ..default()
                        },
                    ));

                    parent.spawn((
                        Name::new("RightEye"),
                        Mesh3d(eye_mesh),
                        MeshMaterial3d(eye_mat),
                        Transform {
                            translation: Vec3::new(
                                0.18,
                                new_actor.capsule_half_height,
                                -new_actor.capsule_radius,
                            ),
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

// TODO: This should probably call a different reducer explicitly for chase and attack
fn handle_lmb_combat(
    local_q: Single<&Player, With<LocalPlayer>>,
    remote_q: Query<&Player, With<RemotePlayer>>,
    actions: Res<ActionState<InputAction>>,
    interactions: Query<&PointerInteraction>,
    stdb: SpacetimeDB,
) {
    let just_pressed = actions.just_pressed(&InputAction::LeftClick);
    let pressed = actions.pressed(&InputAction::LeftClick);

    if !just_pressed && !pressed {
        return;
    }

    let Ok(interaction) = interactions.single() else {
        return;
    };
    let Some((entity, _)) = interaction.get_nearest_hit() else {
        return;
    };

    if let Ok(remote_player) = remote_q.get(*entity) {
        // TODO: Move this to a local cache in ECS, maybe on Player?
        let Some(local_actor) = stdb.db().actor().id().find(&local_q.actor_id) else {
            return;
        };
        // Don't send a new request if this is the same actor
        if let MoveIntent::Actor(actor_id) = &local_actor.move_intent {
            println!("Checking...{:?}={:?}", actor_id, &local_q.actor_id);
            if actor_id == &local_q.actor_id {
                return;
            }
        }
        match stdb
            .reducers()
            .request_move(MoveIntent::Actor(remote_player.actor_id))
        {
            Ok(_) => println!("COMBAT CHASE: {:?}", remote_player.actor_id),
            Err(e) => println!("Error: {}", e),
        }
    }
}
fn handle_lmb_ability(
    actions: Res<ActionState<InputAction>>,
    interactions: Query<&PointerInteraction>,
) {
    let just_pressed = actions.just_pressed(&InputAction::LeftClick);
    let pressed = actions.pressed(&InputAction::LeftClick);
    if !just_pressed && !pressed {
        return;
    }

    let Ok(interaction) = interactions.single() else {
        return;
    };
    let Some((entity, hit)) = interaction.get_nearest_hit() else {
        return;
    };
}

fn handle_lmb_movement(
    mut last_sent_at: ResMut<LastDirectionSentAt>,
    ground_q: Query<&Ground>,
    actions: Res<ActionState<InputAction>>,
    interactions: Query<&PointerInteraction, Without<LocalPlayer>>,
    stdb: SpacetimeDB,
) {
    let just_pressed = actions.just_pressed(&InputAction::LeftClick);
    let pressed = actions.pressed(&InputAction::LeftClick);
    let just_released = actions.just_released(&InputAction::LeftClick);

    if !just_pressed && !pressed && !just_released {
        return;
    }

    let Ok(interaction) = interactions.single() else {
        return;
    };
    let Some((entity, hit)) = interaction.get_nearest_hit() else {
        return;
    };

    if !ground_q.contains(*entity) {
        return;
    }
    let Some(pos) = hit.position else {
        return;
    };

    if just_pressed {
        match stdb.reducers().request_move(MoveIntent::Point(pos.into())) {
            Ok(_) => println!("JUST PRESSED: {:?}", pos),
            Err(e) => println!("Error: {}", e),
        }
    } else if just_released {
        // Reset the "Direct Movement" tracker so the next click feels fresh
        last_sent_at.duration = Duration::ZERO;
        last_sent_at.position = Vec3::ZERO;

        match stdb
            .reducers()
            // TODO: path finding then pass vec to MoveIntent::Path
            .request_move(MoveIntent::Point(pos.into()))
        {
            Ok(_) => println!("JUST RELEASED: {:?}", pos),
            Err(e) => println!("Error: {}", e),
        }
    } else if pressed {
        let held_dur = actions.current_duration(&InputAction::LeftClick);
        if held_dur < Duration::from_millis(150) {
            return;
        }
        let timer_ready = held_dur == Duration::ZERO
            || held_dur.saturating_sub(last_sent_at.duration) >= DIRECTIONAL_MOVEMENT_INTERVAL;

        let distance_ready = last_sent_at.position.distance_squared(pos) > 0.01;

        if !timer_ready || !distance_ready {
            return;
        }

        match stdb.reducers().request_move(MoveIntent::Point(pos.into())) {
            Ok(_) => {
                last_sent_at.position = pos;
                last_sent_at.duration = held_dur;
                println!("PRESSED: {:?}", pos);
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}

fn handle_enter_world(
    current_cursor: ResMut<CurrentCursor>,
    keys: Res<ButtonInput<KeyCode>>,
    stdb: SpacetimeDB,
) {
    if keys.just_pressed(KeyCode::Space) {
        match stdb.reducers().enter_world() {
            Ok(_) => println!("Called enter world without immediate failure"),
            Err(err) => println!("Immediate failure when calling enter world: {}", err),
        }
    } else if keys.just_pressed(KeyCode::Digit1) {
        set_cursor_to_default(current_cursor);
    } else if keys.just_pressed(KeyCode::Digit2) {
        set_cursor_to_ability(current_cursor);
    } else if keys.just_pressed(KeyCode::Digit3) {
        set_cursor_to_combat(current_cursor);
    }
}

fn sync(
    mut transform_query: Query<&mut NetworkTransform, With<Player>>,
    mut messages: ReadUpdateMessage<Actor>,
    stdb: SpacetimeDB,
    actor_entity_mapping: Res<ActorEntityMapping>,
) {
    for msg in messages.read() {
        let Some(actor) = stdb.db().actor().id().find(&msg.new.id) else {
            continue;
        };
        let Some(bevy_entity) = actor_entity_mapping.0.get(&actor.id) else {
            continue;
        };
        if let Ok(mut network_transform) = transform_query.get_mut(*bevy_entity) {
            network_transform.rotation = actor.rotation.into();
            network_transform.scale = actor.scale.into();
            network_transform.translation = actor.translation.into();
        }
    }
}

pub fn interpolate(
    time: Res<Time>,
    mut transform_q: Query<(&mut Transform, &NetworkTransform), With<Player>>,
) {
    let delta_time = time.delta_secs();
    transform_q
        .par_iter_mut()
        .for_each(|(mut transform, network_transform)| {
            transform.translation.smooth_nudge(
                &network_transform.translation.into(),
                18.0,
                delta_time,
            );
            transform.rotation = transform
                .rotation
                .slerp(network_transform.rotation, 1.0 - (-25.0 * delta_time).exp());

            transform.scale = network_transform.scale;
        });
}
