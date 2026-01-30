#![allow(dead_code)]

use crate::module_bindings::{
    DbConnection, MoveIntentData, Reducer, RemoteModule, RemoteReducers,
    request_move_reducer::request_move,
};
use bevy_spacetimedb::RegisterReducerMessage;
use spacetimedb_sdk::ReducerEvent;

#[derive(Debug, RegisterReducerMessage)]
pub struct RequestMove {
    pub event: ReducerEvent<Reducer>,
    pub intent: MoveIntentData,
}

// #[derive(Debug, RegisterReducerMessage)]
// pub struct EnterWorld {
//     pub event: ReducerEvent<Reducer>,
// }

// #[derive(Debug, RegisterReducerMessage)]
// pub struct LeaveWorld {
//     pub event: ReducerEvent<Reducer>,
// }
