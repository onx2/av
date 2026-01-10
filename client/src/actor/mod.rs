mod interpolate;
mod replication;

use bevy::{platform::collections::HashMap, prelude::*};

use crate::module_bindings::MoveIntent;

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(NetworkTransformDataEntityMapping::default());
    app.add_systems(
        PreUpdate,
        (
            replication::on_actor_deleted,
            replication::on_actor_inserted,
        ),
    );
    app.add_systems(
        PostUpdate,
        (
            replication::sync_transform,
            replication::sync_move_intent,
            interpolate::interpolate,
        )
            .chain(),
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
pub struct LocalActor {
    pub id: u64,
    pub move_intent: MoveIntent,
}

#[derive(Component)]
pub struct RemoteActor;
