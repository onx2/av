use crate::{module_bindings::ActiveCharacterRow, server::SpacetimeDB};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_spacetimedb::{ReadDeleteMessage, ReadInsertMessage};
use shared::Owner;

/// Marker to ensure we only attach active-character visuals once per entity.
#[derive(Component, Debug)]
pub struct ActiveCharacterVisuals;

#[derive(Resource, Default)]
pub struct OwnerEntityMapping(pub HashMap<Owner, Entity>);

#[derive(Component, Debug)]
pub struct OwnerEntity(pub Owner);

#[derive(Component, Debug)]
pub struct LocalOwner;

#[derive(Component, Debug)]
pub struct RemoteOwner;

/// Ensures there is a Bevy `Entity` for the given `owner`, regardless of message ordering.
///
/// This is the common pattern for replication timing issues:
/// - Any table insert/update handler can call this first.
/// - It guarantees a stable `Owner -> Entity` mapping exists.
/// - It spawns a minimal entity (only `OwnerEntity`) when needed.
pub fn ensure_owner_entity(
    commands: &mut Commands,
    oe_mapping: &mut OwnerEntityMapping,
    owner: Owner,
) -> Entity {
    if let Some(&entity) = oe_mapping.0.get(&owner) {
        return entity;
    }

    let entity = commands
        .spawn((
            OwnerEntity(owner),
            // Hidden until we have a valid transform. TODO: this might not be necessary once assets for the character are used.
            Visibility::Hidden,
        ))
        .id();
    oe_mapping.0.insert(owner, entity);
    entity
}

/// Set local/remote tags when we know whether the `owner` is local.
///
/// Safe to call multiple times; inserts are idempotent.
pub fn ensure_local_remote_tags(commands: &mut Commands, entity: Entity, is_local: bool) {
    if is_local {
        commands.entity(entity).insert(LocalOwner);
    } else {
        commands.entity(entity).insert(RemoteOwner);
    }
}

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(OwnerEntityMapping::default());
    app.add_systems(
        Update,
        (
            on_active_character_inserted,
            on_active_character_deleted,
            on_monster_instance_inserted,
        ),
    );
}

fn on_active_character_deleted(
    mut commands: Commands,
    mut oe_mapping: ResMut<OwnerEntityMapping>,
    mut msgs: ReadDeleteMessage<ActiveCharacterRow>,
) {
    for msg in msgs.read() {
        if let Some(bevy_entity) = oe_mapping.0.remove(&msg.row.owner) {
            commands.entity(bevy_entity).despawn();
        }
    }
}

fn on_active_character_inserted(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut msgs: ReadInsertMessage<ActiveCharacterRow>,
    mut oe_mapping: ResMut<OwnerEntityMapping>,
    stdb: SpacetimeDB,
    visuals_q: Query<(), With<ActiveCharacterVisuals>>,
) {
    for msg in msgs.read() {
        let is_local = msg.row.identity == stdb.identity();
        let owner = msg.row.owner;

        // Ensure the base entity exists even if other tables arrive first/last.
        let entity = ensure_owner_entity(&mut commands, &mut oe_mapping, owner);
        ensure_local_remote_tags(&mut commands, entity, is_local);

        // Attach visuals only once per entity.
        if visuals_q.get(entity).is_err() {
            let base_color = if is_local {
                Color::linear_rgb(0.2, 0.9, 0.8)
            } else {
                Color::linear_rgb(0.9, 0.2, 0.2)
            };

            // Don't insert `Transform` / `NetTransform` here.
            // Those are owned by transform replication (insert/update messages).
            commands
                .entity(entity)
                .insert((
                    ActiveCharacterVisuals,
                    Mesh3d(meshes.add(Mesh::from(Capsule3d {
                        radius: 0.3,
                        half_length: 0.85,
                    }))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color,
                        ..default()
                    })),
                ))
                .with_children(|parent| {
                    let eye_mesh = meshes.add(Mesh::from(Sphere { radius: 0.12 }));
                    let eye_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 1.0, 1.0),
                        ..default()
                    });

                    let x = 0.18;
                    let y = 0.85;
                    let z = -0.3;

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
        }

        println!("on_active_character_inserted: {:?}", owner);
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
