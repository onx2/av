use crate::actor::LocalActor;
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, add_camera);
    app.add_systems(PostUpdate, follow_player);
}

const CAMERA_OFFSET_GLOBAL: Vec3 = Vec3::new(0.0, 25.0, -10.0);
const CAMERA_DECAY_RATE: f32 = 24.0;

fn add_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(CAMERA_OFFSET_GLOBAL).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn follow_player(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    local_q: Query<&Transform, (With<LocalActor>, Without<Camera3d>)>,
    time: Res<Time>,
) {
    let Ok(mut cam_tf) = camera_query.single_mut() else {
        return;
    };
    let Ok(local_tf) = local_q.single() else {
        return;
    };

    let target = local_tf.translation + CAMERA_OFFSET_GLOBAL;
    cam_tf
        .translation
        .smooth_nudge(&target, CAMERA_DECAY_RATE, time.delta_secs());
}
