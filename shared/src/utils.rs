use super::constants::*;
use nalgebra as na;
use std::f32::consts::TAU;

pub fn yaw_from_xz(xz: &na::Vector2<f32>) -> Option<f32> {
    if xz.norm_squared() > YAW_EPS {
        return Some((-xz[0]).atan2(-xz[1]));
    }

    None
}

/// Quantize yaw (radians) into a single byte.
///
/// Convention:
/// - input: yaw in radians (any range; e.g. [-π, π] or [0, 2π))
/// - output: `u8` in 0..=255 representing [0, 2π) in 256 uniform steps
pub fn yaw_to_u8(yaw_radians: f32) -> u8 {
    const SCALE: f32 = 256.0 / TAU;

    // 1. Multiply to get range approx [-128.0, 128.0]
    // 2. Cast to i32 to handle the negative sign
    // 3. Cast to u8 to truncate to the 0..255 range
    (yaw_radians * SCALE) as i32 as u8
}

/// Dequantize `u8` yaw back into radians in [0, 2π).
pub fn yaw_from_u8(code: u8) -> f32 {
    (code as f32) * (TAU / 256.0)
}

pub trait UtilMath {
    fn sq(self) -> Self;
}

impl<T> UtilMath for T
where
    T: std::ops::Mul<Output = T> + Copy,
{
    fn sq(self) -> Self {
        self * self
    }
}

pub fn to_planar(vec: &na::Vector3<f32>) -> na::Vector2<f32> {
    na::Vector2::new(vec.x, vec.z)
}

/// Planar (XZ) distance squared between two world positions (meters^2).
pub fn planar_distance_sq(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> f32 {
    (b.x - a.x).sq() + (b.z - a.z).sq()
}

/// Are two positions within a planar movement range (meters)?
pub fn is_move_too_far(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> bool {
    planar_distance_sq(a, b) > MAX_INTENT_DISTANCE_SQ
}

/// Are two positions within a planar acceptance radius (meters)?
pub fn is_move_too_close(a: &na::Vector3<f32>, b: &na::Vector3<f32>) -> bool {
    planar_distance_sq(a, b) <= SMALLEST_REQUEST_DISTANCE_SQ
}

/// Encodes world position (x, z) into a compact 32-bit cell ID.
///
/// - Adds `WORLD_OFFSET` to shift negative coords into positive range.
/// - Divides by `CELL_SIZE` to get grid indices (floored).
/// - Casts to u16 (truncates fractional part).
/// - Packs: grid_x into high 16 bits (<< 16), grid_z into low 16 bits (|).
///
/// Result: Unique u32 ID with X-major ordering (x high, z low).
pub fn encode_cell_id(x: f32, z: f32) -> u32 {
    let grid_x = ((x + WORLD_OFFSET) / CELL_SIZE) as u16;
    let grid_z = ((z + WORLD_OFFSET) / CELL_SIZE) as u16;
    (u32::from(grid_x) << 16) | u32::from(grid_z)
}

/// Decodes a 32-bit cell ID into the world position (x, z) of the cell's minimum (bottom-left) corner.
///
/// - Extracts grid coordinates: x from high 16 bits, z from low 16 bits.
/// - Multiplies by `CELL_SIZE` to get offset position.
/// - Subtracts `WORLD_OFFSET` to revert the encoding shift.
///
/// Returns [x, z] in world units.
pub fn decode_cell_id(id: u32) -> [f32; 2] {
    let grid_x = (id >> 16) as u16;
    let grid_z = (id & 0xFFFF) as u16;
    [
        f32::from(grid_x) * CELL_SIZE - WORLD_OFFSET,
        f32::from(grid_z) * CELL_SIZE - WORLD_OFFSET,
    ]
}

/// Returns the 9 cell IDs forming a 3x3 Area of Interest (AOI) block around the given center cell.
///
/// Layout (top-down view, +Z = North):
///
/// [0] North-West | [1] North     | [2] North-East
/// ------------------------------------------------
/// [3] West       | [4] Center    | [5] East
/// ------------------------------------------------
/// [6] South-West | [7] South     | [8] South-East
///
/// Index 4 is always the input `id`. Neighbors use saturating arithmetic to clamp at u16 bounds (0..65535).
pub fn get_aoi_block(id: u32) -> [u32; 9] {
    // Cell ID format: [x: u16 (bits 31-16)] [z: u16 (bits 15-0)]
    let x = (id >> 16) as u16; // Extract grid X from low 16 bits after right shift
    let z = (id & 0xFFFF) as u16; // Extract grid Z from low 16 bits after bitwise AND

    // Neighbor coordinates (clamp to valid range)
    let xw = x.saturating_sub(1); // West
    let xe = x.saturating_add(1); // East
    let zn = z.saturating_add(1); // North
    let zs = z.saturating_sub(1); // South

    // Pre-shift X values to high 16 bits
    let x_shifted = u32::from(x) << 16;
    let xw_shifted = u32::from(xw) << 16;
    let xe_shifted = u32::from(xe) << 16;

    // Reconstruct neighbor IDs via OR (low 16 bits = Z, high already shifted)
    [
        xw_shifted | u32::from(zn), // NW
        x_shifted | u32::from(zn),  // N
        xe_shifted | u32::from(zn), // NE
        xw_shifted | u32::from(z),  // W
        id,                         // Center
        xe_shifted | u32::from(z),  // E
        xw_shifted | u32::from(zs), // SW
        x_shifted | u32::from(zs),  // S
        xe_shifted | u32::from(zs), // SE
    ]
}
