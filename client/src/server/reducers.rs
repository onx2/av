#![allow(dead_code)]

use crate::module_bindings::{
    DbConnection, MoveIntentData, Reducer, RemoteModule, RemoteReducers,
    enter_game_reducer::enter_game, request_move_reducer::request_move,
};
use bevy_spacetimedb::RegisterReducerMessage;
use shared::OwnerId;
use spacetimedb_sdk::ReducerEvent;

#[derive(Debug, RegisterReducerMessage)]
pub struct RequestMove {
    pub event: ReducerEvent<Reducer>,
    pub intent: MoveIntentData,
}

#[derive(Debug, RegisterReducerMessage)]
pub struct EnterGame {
    pub event: ReducerEvent<Reducer>,
    pub character_id: OwnerId,
}

// #[derive(Debug, RegisterReducerMessage)]
// pub struct LeaveWorld {
//     pub event: ReducerEvent<Reducer>,
// }
