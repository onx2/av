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
pub const MAX_INTENT_PATH_LEN: usize = 20;

/// Minimum planar motion required to update yaw (meters per tick).
pub const YAW_EPS: f32 = 1.0e-6;

/// Size of one grid cell in world units (meters).
/// All cells are square
pub const CELL_SIZE: f32 = 10.0;
pub const INV_CELL_SIZE: f32 = 1.0 / CELL_SIZE;

/// Offset applied when converting world positions to grid coordinates.
/// Shifts the world origin so that grid (0,0) corresponds to world position (-32768, -32768).
/// Allows unsigned u16 grid coords to cover a world range of ~655360 units (±327680).
pub const WORLD_OFFSET: f32 = 32768.0;

pub const GRAVITY: f32 = -23.81;
pub const TERMINAL_VELOCITY: f32 = GRAVITY * 4.0;
