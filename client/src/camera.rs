use crate::actor::LocalActor;
use bevy::{
    camera::Exposure,
    pbr::{AtmosphereMode, AtmosphereSettings},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, add_camera);
    app.add_systems(PostUpdate, follow_player);
}

const CAMERA_OFFSET_GLOBAL: Vec3 = Vec3::new(0.0, 25.0, -10.0);
const CAMERA_DECAY_RATE: f32 = 44.0;

fn add_camera(mut commands: Commands) {
    commands.spawn((
        Exposure { ev100: 16.0 },
        bevy::core_pipeline::tonemapping::Tonemapping::AcesFitted,
        Camera3d::default(),
        Transform::from_translation(CAMERA_OFFSET_GLOBAL).looking_at(Vec3::ZERO, Vec3::Y),
        DistanceFog {
            color: Color::srgba(0.35, 0.48, 0.66, 1.0),
            directional_light_color: Color::srgba(1.0, 0.95, 0.85, 0.5),
            directional_light_exponent: 30.0,
            falloff: FogFalloff::from_visibility_colors(
                1000.0, // Fog distance
                Color::srgb(0.35, 0.5, 0.66),
                Color::srgb(0.8, 0.8, 0.7),
            ),
        },
        AtmosphereSettings {
            rendering_method: AtmosphereMode::Raymarched,
            ..default()
        },
    ));
}

fn follow_player(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    local_owner: Single<&Transform, (With<LocalActor>, Without<Camera3d>)>,
    time: Res<Time>,
) {
    let Ok(mut cam_tf) = camera_query.single_mut() else {
        return;
    };

    let target = local_owner.translation + CAMERA_OFFSET_GLOBAL;
    cam_tf
        .translation
        .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());
}
