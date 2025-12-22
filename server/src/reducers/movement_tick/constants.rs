//! Movement tick constants.
//!
//! Centralizes tick rates and dt clamps so they can be reused consistently across:
//! - `player_tick`
//! - `non_player_tick`
//!
//! These ticks are intended for "reasonable collision detection" rather than deterministic physics,
//! so they use clamped variable timesteps.

/// Player movement tick frequency (Hz).
pub const PLAYER_TICK_HZ: i64 = 30;

/// Non-player movement tick frequency (Hz).
pub const NON_PLAYER_TICK_HZ: i64 = 10;

/// Max dt (seconds) for player movement updates.
///
/// Players get a tighter clamp to keep movement responsive and avoid large jumps after stalls.
pub const MAX_PLAYER_DT_S: f32 = 0.10;

/// Max dt (seconds) for non-player movement updates.
///
/// Non-players get a looser clamp to tolerate stalls without accumulator catch-up spirals.
pub const MAX_NON_PLAYER_DT_S: f32 = 0.25;
