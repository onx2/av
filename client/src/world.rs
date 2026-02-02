use bevy::prelude::*;
use bevy_spacetimedb::ReadInsertMessage;

use crate::module_bindings::{ColliderShape, WorldStatic};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(Update, load_world);
}

#[derive(Component)]
pub struct Ground;

fn setup(mut commands: Commands) {
    println!("World setup");

    // light
    commands.spawn((
        DirectionalLight {
            illuminance: 80_000.0,
            shadows_enabled: true,
            ..default()
        },
        // Orientation: Looking down from the sky
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::new(1.0, -1.0, 1.0), Vec3::Y),
    ));
}

fn load_world(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<WorldStatic>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for msg in msgs.read() {
        println!("WorldStatic: {:?}", msg.row.id);
        let world_static = msg.row.clone();

        match world_static.shape {
            ColliderShape::Plane(_) => {
                commands.spawn((
                    Ground,
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
