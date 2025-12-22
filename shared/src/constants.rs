use std::time::Duration;

/// How often (Hz) to run the server-side idle-player overlap separation tick.
///
/// This tick is intended to resolve minor capsule overlaps for players that are not moving
/// (e.g. spawn stacking) without doing expensive full movement simulation.
///
/// Notes:
/// - This should be low frequency (e.g. 10Hz) since it's a corrective/nudging pass.
/// - The implementation should only process idle players and only consider neighbors in nearby cells.
pub const IDLE_OVERLAP_PUSH_TICK_HZ: i64 = 10;

/// Small extra separation distance (meters) added on top of the combined capsule radii.
///
/// This helps avoid persistent re-overlap due to float/quantization and ensures actors end up
/// slightly separated after a push.
pub const OVERLAP_PUSH_SKIN_M: f32 = 0.02;

/// Maximum planar push distance (meters) applied to a single idle player per tick.
///
/// Prevents large teleports in pathological cases (e.g. many actors stacked).
pub const OVERLAP_PUSH_MAX_STEP_M: f32 = 0.25;

/// Hard limit on how many neighbor candidate actors we consider per idle player.
///
/// This is a safety valve for crowded cells. If exceeded, the push reducer can skip or
/// only process a subset to avoid performance cliffs.
pub const OVERLAP_PUSH_MAX_NEIGHBORS_PER_ACTOR: usize = 64;

/// Air-control multiplier for planar (XZ) movement while airborne.
///
/// Convention:
/// - 1.0 = full ground control in air (arcade / very floaty)
/// - 0.0 = no air control (current "abrupt stop" behavior)
///
/// Typical values: 0.1 .. 0.4
pub const AIR_CONTROL_MULTIPLIER: f32 = 0.3;

/// Quantization step for the Y axis in meters.
///
/// Used anywhere we store vertical position in a quantized integer form.
/// Current convention: `y_q: i16` stores `y_meters / Y_QUANTIZE_STEP_M`.
pub const Y_QUANTIZE_STEP_M: f32 = 0.01;

/// How frequently, in milliseconds, to send directional movement updates to the server.
pub const DIRECTIONAL_MOVEMENT_INTERVAL: Duration = Duration::from_millis(50);

/// The smallest distance, squared, difference between two move requests allowed
pub const SMALLEST_REQUEST_DISTANCE_SQ: f32 = 0.1;

pub const SMALLEST_MOVE_DISTANCE_SQ: f32 = 0.0001;

/// Default server-side maximum allowed movement intent distance (meters).
pub const MAX_INTENT_DISTANCE_SQ: f32 = 100.0 * 100.0;

pub const MAX_INTENT_PATH_LEN: usize = 20;

/// Minimum planar motion required to update yaw (meters per tick).
pub const YAW_EPS: f32 = 1.0e-6;

/// Size of one grid cell in world units (meters).
/// All cells are square.
pub const CELL_SIZE: f32 = 5.0;

/// Offset applied when converting world positions to grid coordinates.
///
/// This is used by `encode_cell_id()` / `decode_cell_id()`:
/// - Encoding: `grid = floor((pos + WORLD_OFFSET) / CELL_SIZE)` cast to `u16`
/// - Decoding: `pos_min = grid * CELL_SIZE - WORLD_OFFSET`
///
/// With `u16` grid coords (0..=65535), the representable world span per axis is:
/// - total span: `65536 * CELL_SIZE` meters
/// - minimum position: `-WORLD_OFFSET`
/// - maximum position (cell minimum): `(65535 * CELL_SIZE) - WORLD_OFFSET`
///
/// Note: this range is generally not symmetric around 0 unless `WORLD_OFFSET` is chosen accordingly.
pub const WORLD_OFFSET: f32 = 32768.0;
