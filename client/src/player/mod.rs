use crate::module_bindings::MoveIntent;
use bevy::{platform::collections::HashMap, prelude::*};

mod input;
mod interpolate;
mod replication;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<input::LastDirectionSentAt>();
    app.insert_resource(ActorEntityMapping::default());

    // STDB replication systems
    app.add_systems(
        PreUpdate,
        (
            replication::on_actor_inserted,
            replication::on_actor_deleted,
        ),
    );

    // Gameplay/input systems
    app.add_systems(
        Update,
        (input::handle_enter_world, input::handle_lmb_movement),
    );

    // Replication sync should run before interpolation, so we always smooth toward the latest
    // received snapshot in the same frame.
    app.add_systems(PostUpdate, (replication::sync, interpolate::interpolate));
}

/// Used to tie the server entity id to the local bevy entity
#[derive(Resource, Default)]
pub struct ActorEntityMapping(pub HashMap<u64, Entity>);

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct NetworkActor {
    // pub actor_id: u64,
    pub translation: Vec3,
    pub rotation: Quat,
    // pub move_intent: MoveIntent,
}

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component)]
pub struct RemotePlayer;
