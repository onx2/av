mod extrapolate;
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
            replication::sync_aoi_actor,
            replication::sync_transform,
        ),
    );
    // app.add_systems(Update, extrapolate::extrapolate_movement);
}

/// Used to tie the server TransformData ID to the local bevy entity for efficient lookups when reconciling from network
#[derive(Resource, Default)]
pub struct NetworkTransformDataEntityMapping(pub HashMap<u64, Entity>);

/// The server's network transform for a given actor, cached as a bevy component on entities
#[derive(Component)]
pub struct NetworkTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

/// A client/server shared cache for an actor/entity's movement data. Useful for extrapolation.
/// Local client directly sets move_intent from input allowing for "prediction" while remotes extrapolate using ~1 frame lag
#[derive(Component)]
pub struct MovementData {
    pub move_intent: MoveIntent,
    pub grounded: bool,
    pub movement_speed: f32,
}

/// Marker component for the locally controlled actor/entity (represents the entity controlled by the person behind the keyboard, YOU!)
#[derive(Component)]
pub struct LocalActor;

/// Marker component for remotely controlled actor/entity (other players, monsters, NPCs)
#[derive(Component)]
pub struct RemoteActor;
