// mod reducers {
//     mod connection;
//     pub mod enter_world;
//     pub mod leave_world;
//     pub(crate) mod movement_tick;
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

use spacetimedb::*;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("Database initializing...");
    foo::ProgressionSystem::regenerate(ctx);
    Ok(())
}
