#![allow(dead_code)]

use crate::module_bindings::{
    DbConnection, MoveIntentData, Reducer, RemoteModule, RemoteReducers,
    cancel_move_reducer::cancel_move, create_character_reducer::create_character,
    enter_game_reducer::enter_game, request_move_reducer::request_move,
};
use bevy_spacetimedb::RegisterReducerMessage;
use spacetimedb_sdk::ReducerEvent;

#[derive(Debug, RegisterReducerMessage)]
pub struct RequestMove {
    pub event: ReducerEvent<Reducer>,
    pub intent: MoveIntentData,
    pub client_intent_seq: u32,
}

#[derive(Debug, RegisterReducerMessage)]
pub struct EnterGame {
    pub event: ReducerEvent<Reducer>,
    pub character_id: u32,
}

#[derive(Debug, RegisterReducerMessage)]
pub struct CreateCharacter {
    pub event: ReducerEvent<Reducer>,
    pub name: String,
}

#[derive(Debug, RegisterReducerMessage)]
pub struct CancelMove {
    pub event: ReducerEvent<Reducer>,
}

// #[derive(Debug, RegisterReducerMessage)]
// pub struct LeaveWorld {
//     pub event: ReducerEvent<Reducer>,
// }
