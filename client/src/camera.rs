use bevy::prelude::*;

use crate::player::LocalPlayer;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, add_camera);
    app.add_systems(PostUpdate, follow_player);
}

const CAMERA_OFFSET_GLOBAL: Vec3 = Vec3::new(0.0, 25.0, -10.0);
const CAMERA_DECAY_RATE: f32 = 12.0;

fn add_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_OFFSET_GLOBAL).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn follow_player(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    player_query: Query<&Transform, (With<LocalPlayer>, Without<Camera3d>)>,
    time: Res<Time>,
) {
    let Ok(mut cam_tf) = camera_query.single_mut() else {
        return;
    };
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    let target = player_tf.translation + CAMERA_OFFSET_GLOBAL;
    cam_tf
        .translation
        .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());
}
