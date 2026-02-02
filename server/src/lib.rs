pub mod active_character;
pub mod character;
pub mod monster;
pub mod monster_instance;
pub mod movement;
pub mod npc;
pub mod player;
pub mod primitives;
pub mod progression;
pub mod stat;
pub mod status_flags;
pub mod tags;
pub mod transform;
pub mod util;
pub mod world_static;

pub use active_character::*;
pub use character::*;
pub use monster::*;
pub use monster_instance::*;
pub use movement::*;
pub use npc::*;
pub use player::*;
pub use primitives::*;
pub use progression::*;
pub use stat::*;
pub use status_flags::*;
pub use transform::*;
pub use util::*;
pub use world_static::*;

use spacetimedb::*;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("Database initializing...");
    regenerate_static_world(ctx);
    init_movement_tick(ctx);
    init_health_and_mana_regen(ctx);
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);
    PlayerRow::connect(ctx);
}

#[spacetimedb::reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    log::info!("Client disconnected: {:?}", ctx.sender);
    PlayerRow::disconnect(ctx);
}
