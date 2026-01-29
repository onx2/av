pub mod active_character;
pub mod character;
pub mod monster;
pub mod monster_instance;
pub mod move_intent;
pub mod movement_state;
pub mod npc;
pub mod player;
pub mod primitives;
pub mod process_move_intent;
pub mod progression;
pub mod request_move;
pub mod stat;
pub mod status_flags;
pub mod tags;
pub mod transform;

pub use active_character::*;
pub use character::*;
pub use monster::*;
pub use monster_instance::*;
pub use move_intent::*;
pub use movement_state::*;
pub use npc::*;
pub use player::*;
pub use primitives::*;
pub use process_move_intent::*;
pub use progression::*;
pub use stat::*;
pub use status_flags::*;
pub use transform::*;

use spacetimedb::*;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("Database initializing...");
    init_process_move_intent(ctx);
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
