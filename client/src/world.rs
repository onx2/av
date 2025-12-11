use bevy::prelude::*;

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

    commands.spawn((
        // Ground,
        Pickable::default(),
        Transform::from_xyz(0., 0., 0.),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50., 50.).build())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.2, 0.3, 0.25),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            ..default()
        })),
    ));

    // cube
    commands.spawn((
        Pickable::default(),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(5.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}

fn load_world() {
    // Todo, read in the world static objects from spacetime and render
}
