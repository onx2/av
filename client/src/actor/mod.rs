mod interpolate;
mod replication;

use bevy::{platform::collections::HashMap, prelude::*};

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(NetworkTransformDataEntityMapping::default());
    app.add_systems(
        PreUpdate,
        (
            replication::on_actor_deleted,
            replication::on_actor_inserted,
        ),
    );

    // Replication sync should run before interpolation, so we always smooth toward the latest
    // received snapshot in the same frame.
    app.add_systems(
        PostUpdate,
        (replication::sync_transform, interpolate::interpolate),
    );
}

/// Used to tie the server TransformData ID to the local bevy entity for efficient lookups when reconciling from network
#[derive(Resource, Default)]
pub struct NetworkTransformDataEntityMapping(pub HashMap<u64, Entity>);

#[derive(Component)]
pub struct NetworkTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

#[derive(Component)]
pub struct LocalActor;

#[derive(Component)]
pub struct RemoteActor;
