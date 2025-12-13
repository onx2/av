use bevy::prelude::*;
use bevy_spacetimedb::ReadInsertMessage;

use crate::{
    module_bindings::{ColliderShape, WorldStatic},
    server::SpacetimeDB,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(Update, load_world);
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("World setup");

    // commands.spawn((
    //     // Ground,
    //     Pickable::default(),
    //     Transform::from_xyz(0., 0., 0.),
    //     Mesh3d(meshes.add(Plane3d::default().mesh().size(50., 50.).build())),
    //     MeshMaterial3d(materials.add(StandardMaterial {
    //         base_color: Color::linear_rgb(0.2, 0.3, 0.25),
    //         perceptual_roughness: 1.0,
    //         metallic: 0.0,
    //         ..default()
    //     })),
    // ));

    // // cube
    // commands.spawn((
    //     Pickable::default(),
    //     Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
    //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    //     Transform::from_xyz(5.0, 0.5, 0.0),
    // ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}

fn load_world(
    stdb: SpacetimeDB,
    mut commands: Commands,
    mut msgs: ReadInsertMessage<WorldStatic>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for msg in msgs.read() {
        println!("WorldStatic: {:?}", msg.row.id);
        let world_static = msg.row.clone();

        match world_static.shape {
            ColliderShape::Plane(val) => {
                commands.spawn((
                    // Ground,
                    Pickable::default(),
                    Transform {
                        rotation: world_static.rotation.into(),
                        translation: world_static.translation.into(),
                        scale: world_static.scale.clone().into(),
                    },
                    Mesh3d(
                        meshes.add(
                            Plane3d::default()
                                .mesh()
                                .size(world_static.scale.x, world_static.scale.z)
                                .build(),
                        ),
                    ),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::linear_rgb(0.2, 0.3, 0.25),
                        perceptual_roughness: 1.0,
                        metallic: 0.0,
                        ..default()
                    })),
                ));
            }
            ColliderShape::Cuboid(val) => {
                commands.spawn((
                    // Ground,
                    Pickable::default(),
                    Transform {
                        rotation: world_static.rotation.into(),
                        translation: world_static.translation.into(),
                        scale: world_static.scale.into(),
                    },
                    Mesh3d(meshes.add(Cuboid::new(val.x * 2., val.y * 2., val.z * 2.))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::linear_rgb(0.8, 0.1, 0.15),
                        perceptual_roughness: 1.0,
                        metallic: 0.0,
                        ..default()
                    })),
                ));
            }
            _ => unimplemented!("This shouldn't be reached"),
        }
    }
}
