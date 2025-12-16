#![allow(dead_code)]

use crate::module_bindings::{
    DbConnection, MoveIntent, Reducer, RemoteModule, RemoteReducers,
    enter_world_reducer::enter_world, leave_world_reducer::leave_world,
    request_move_reducer::request_move, spawn_fake_remotes_reducer::spawn_fake_remotes,
};
use bevy_spacetimedb::RegisterReducerMessage;
use spacetimedb_sdk::ReducerEvent;

#[derive(Debug, RegisterReducerMessage)]
pub struct RequestMove {
    pub event: ReducerEvent<Reducer>,
    pub move_intent: MoveIntent,
}

#[derive(Debug, RegisterReducerMessage)]
pub struct EnterWorld {
    pub event: ReducerEvent<Reducer>,
}

#[derive(Debug, RegisterReducerMessage)]
pub struct LeaveWorld {
    pub event: ReducerEvent<Reducer>,
}

#[derive(Debug, RegisterReducerMessage)]
pub struct SpawnFakeRemotes {
    pub event: ReducerEvent<Reducer>,
    pub count: u32,
}
