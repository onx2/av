//! Movement tick module (folder-based).
//!
//! Layout:
//! - `constants.rs`: tick rates + dt clamps
//! - `utils.rs`: shared per-actor movement step + shared iteration loop
//! - `player_tick.rs`: player scheduled timer + reducer
//! - `non_player_tick.rs`: non-player scheduled timer + reducer
//!
//! Timers are colocated with their respective reducers, per your preference.
//!
//! This `mod.rs` only wires modules and re-exports the public entrypoints.

mod constants;
mod utils;

mod non_player_tick;
mod player_tick;

/// Initialize both movement tick schedules.
///
/// This should be called from the crate `init` reducer.
pub fn init(ctx: &spacetimedb::ReducerContext) {
    player_tick::init(ctx);
    non_player_tick::init(ctx);
}
