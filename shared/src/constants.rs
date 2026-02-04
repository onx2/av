use std::time::Duration;

/// How frequently, in milliseconds, to send directional movement updates to the server.
pub const DIRECTIONAL_MOVEMENT_INTERVAL: Duration = Duration::from_millis(50);

/// 2π (tau), full rotation in radians.
/// Used for yaw quantization helpers.
pub const TAU: f32 = std::f32::consts::TAU;

/// The smallest distance, squared, difference between two move requests allowed
pub const SMALLEST_REQUEST_DISTANCE_SQ: f32 = 0.1;

/// The smallest distance squared that an actor can move through desired intent
pub const SMALLEST_MOVE_DISTANCE_SQ: f32 = 0.0001;

/// Default server-side maximum allowed movement intent distance (meters).
pub const MAX_INTENT_DISTANCE_SQ: f32 = 100.0 * 100.0;

/// The maximum number of points on a path that are allowed
pub const MAX_INTENT_PATH_LEN: usize = 5;

/// Minimum planar motion required to update yaw (meters per tick).
pub const YAW_EPS: f32 = 1.0e-6;

/// Size of one grid cell in world units (meters).
/// All cells are square
pub const CELL_SIZE: f32 = 50.0;
pub const INV_CELL_SIZE: f32 = 1.0 / CELL_SIZE;

/// Side length (cells per axis) for the square `u16` cell-id grid.
///
/// A `u16` can represent 65536 total cells, so a square grid must be 256×256.
pub const GRID_SIDE: u16 = 256;
pub const GRID_SIDE_F: f32 = GRID_SIDE as f32;

/// Offset applied when converting world positions to grid coordinates.
///
/// For the `u16` cell-id grid, we use a fixed `GRID_SIDE × GRID_SIDE` world. To support negative
/// world coordinates while keeping the cell coordinate range in `[0, GRID_SIDE)`, we shift by
/// half the world span so that world-space `(0, 0)` maps near the center of the grid.
///
/// World span per axis (meters): `GRID_SIDE as f32 * CELL_SIZE`
/// Offset (meters): `world_span / 2`
pub const WORLD_OFFSET: f32 = GRID_SIDE_F * CELL_SIZE * 0.5;

/// Gravity acceleration (meters/second^2). Negative is downward.
pub const GRAVITY_MPS2: f32 = -13.81;

/// Terminal fall speed (meters/second). Negative is downward.
pub const TERMINAL_FALL_SPEED_MPS: f32 = GRAVITY_MPS2 * 3.;

/// Vertical velocity quantization scale (meters/second per 1 `i8` unit).
///
/// Stored vertical velocity (`i8`) represents: `v_mps = v_q as f32 * VERTICAL_VELOCITY_Q_MPS`.
/// Smaller values = finer precision but smaller max representable speed.
///
/// With `0.25`, `i8` covers approximately [-32.0, +31.75] m/s.
pub const VERTICAL_VELOCITY_Q_MPS: f32 = 0.25;

pub const MICROS_60HZ: i64 = 16_666;
pub const MICROS_30HZ: i64 = 33_333;
pub const MICROS_20HZ: i64 = 50_000;
pub const MICROS_10HZ: i64 = 100_000;
pub const MICROS_1HZ: i64 = 1_000_000;
