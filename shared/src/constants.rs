use std::time::Duration;

/// How frequently, in milliseconds, to send directional movement updates to the server.
pub const DIRECTIONAL_MOVEMENT_INTERVAL: Duration = Duration::from_millis(50);

/// The smallest distance, squared, difference between two move requests allowed
pub const SMALLEST_REQUEST_DISTANCE_SQ: f32 = 0.1;

/// Default server-side maximum allowed movement intent distance (meters).
pub const MAX_INTENT_DISTANCE_SQ: f32 = 100.0 * 100.0;

pub const MAX_INTENT_PATH_LEN: usize = 20;

/// Minimum planar motion required to update yaw (meters per tick).
pub const YAW_EPS: f32 = 1.0e-6;
