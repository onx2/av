// mod reducers {
//     mod connection;
//     pub mod enter_world;
//     pub mod leave_world;
// pub(crate) mod movement_tick;
//     pub mod request_move;
// }
// pub mod schema;
// pub mod types;
// mod utils;
// pub mod views;
// mod world;

// use crate::schema::*;
// use reducers::movement_tick::init_movement_tick;
// use spacetimedb::*;

// #[reducer(init)]
// pub fn init(ctx: &ReducerContext) {
//     world::recreate_static_world(ctx);
//     init_movement_tick(ctx);
// }

pub mod foo;

// Macro crate level re-exports
pub use foo::DataTable;

use spacetimedb::*;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("Database initializing...");
    foo::ProgressionSystem::regenerate(ctx);
    foo::init_process_move_intent(ctx);
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);
    // Create a new player if not found
}

#[spacetimedb::reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    log::info!("Client disconnected: {:?}", ctx.sender);
    // Find player and active_character, leave_game if found.
}
