use super::constants::*;
use crate::{WorldStaticDef, collider_from_def};
use nalgebra::{Isometry, Translation3, Vector2, Vector3};
use rapier3d::prelude::{
    BroadPhaseBvh, ColliderSet, IntegrationParameters, NarrowPhase, QueryFilter, QueryPipeline,
    RigidBodySet,
};

pub fn yaw_from_xz(xz: Vector2<f32>) -> Option<f32> {
    if xz.norm_squared() > YAW_EPS {
        return Some((-xz[0]).atan2(-xz[1]));
    }

    None
}

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

/// Quantize radians into a u16 [0, 65535].
pub fn yaw_to_u16(yaw_radians: f32) -> u16 {
    const SCALE: f32 = 65536.0 / TAU;

    // 1. Multiply to get range approx [-32768.0, 32768.0] (if input is -PI to PI)
    // 2. Cast to i32 to handle negative signs via bit wrapping
    // 3. Cast to u16 to truncate to the 0..65535 range
    (yaw_radians * SCALE) as i32 as u16
}

/// Dequantize `u16` yaw back into radians in [0, 2π).
pub fn yaw_from_u16(code: u16) -> f32 {
    const REV_SCALE: f32 = TAU / 65536.0;

    (code as f32) * REV_SCALE
}

/// Returns true if two world positions are within the planar (XZ) acceptance radius.
pub fn is_at_target_planar(current: Vector2<f32>, target: Vector2<f32>) -> bool {
    const CM_SQ: f32 = 1.0e-4;
    (target - current).norm_squared() <= CM_SQ
}

pub fn get_desired_delta(
    current_planar: Vector2<f32>,
    target_planar: Vector2<f32>,
    movement_speed_mps: f32,
    vertical_velocity: u16,
    dt: f32,
) -> Vector3<f32> {
    const GROUND_BIAS_VELOCITY: f32 = -0.125;
    const AIR_CONTROL_REDUCTION: f32 = 0.5;
    const MM_SQ: f32 = 1.0e-6;

    let max_step = movement_speed_mps * dt;
    let dx = target_planar.x - current_planar.x;
    let dz = target_planar.y - current_planar.y;
    let dist_sq = dx * dx + dz * dz;

    let (x, z) = if dist_sq <= MM_SQ {
        (0.0, 0.0)
    } else {
        let dist = dist_sq.sqrt();
        let scale = max_step.min(dist) / dist;
        (dx * scale, dz * scale)
    };

    if vertical_velocity == 0 {
        // Very slight downward bias to help snap to ground on slopes
        [x, GROUND_BIAS_VELOCITY * dt, z].into()
    } else {
        // Air control reduction in planar and gravity.
        [
            x * AIR_CONTROL_REDUCTION,
            -(vertical_velocity as f32) * dt,
            z * AIR_CONTROL_REDUCTION,
        ]
        .into()
    }
}

/// Planar (XZ) distance squared between two world positions (meters^2).
pub fn planar_distance_sq(a: Vector2<f32>, b: Vector2<f32>) -> f32 {
    let x = b.x - a.x;
    let z = b.y - a.y;
    x * x + z * z
}

/// Are two positions within a planar movement range (meters)?
pub fn is_move_too_far(a: Vector2<f32>, b: Vector2<f32>) -> bool {
    planar_distance_sq(a, b) > MAX_INTENT_DISTANCE_SQ
}

/// Are two positions within a planar acceptance radius (meters)?
pub fn is_move_too_close(a: Vector2<f32>, b: Vector2<f32>) -> bool {
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
    let grid_x = ((x + WORLD_OFFSET) * INV_CELL_SIZE) as u16;
    let grid_z = ((z + WORLD_OFFSET) * INV_CELL_SIZE) as u16;
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
    [
        f32::from((id >> 16) as u16) * CELL_SIZE - WORLD_OFFSET,
        // 0xFFFF == u16::MAX, but converts to u32 implicitly
        f32::from((id & 0xFFFF) as u16) * CELL_SIZE - WORLD_OFFSET,
    ]
}

/// Returns the 9 cell IDs forming a 3x3 Area of Interest (AOI) block around the given center cell.
///
/// **NOTE** Its expected to have a buffer of 1 cell around the world so we don't need to worry about the
/// saturating add/sub duplicating values for center and edge.
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
///
/// **Performance**: O(1)
pub fn get_aoi_block(cell_id: u32) -> [u32; 9] {
    // Cell ID format: [x: u16 (bits 31-16)] [z: u16 (bits 15-0)]
    let x = (cell_id >> 16) as u16; // Extract grid X from low 16 bits after right shift
    let z = (cell_id & 0xFFFF) as u16; // Extract grid Z from low 16 bits after bitwise AND

    // Neighbor coordinates (wrap to valid range and avoid branching, saves a few cycles)
    // It is expected we never hit the edge of the grid
    let x_west = x.wrapping_sub(1); // West (X-1)
    let x_east = x.wrapping_add(1); // East (X+1)
    let z_north = z.wrapping_add(1); // North (Z-1)
    let z_south = z.wrapping_sub(1); // South (Z+1)

    // Pre-shift X values to high 16 bits to pack
    // Reconstruct neighbor IDs via OR (low 16 bits = Z, high already shifted)
    [
        (u32::from(x_west) << 16) | u32::from(z_north), // NW
        (u32::from(x) << 16) | u32::from(z_north),      // N
        (u32::from(x_east) << 16) | u32::from(z_north), // NE
        (u32::from(x_west) << 16) | u32::from(z),       // W
        cell_id,                                        // Center
        (u32::from(x_east) << 16) | u32::from(z),       // E
        (u32::from(x_west) << 16) | u32::from(z_south), // SW
        (u32::from(x) << 16) | u32::from(z_south),      // S
        (u32::from(x_east) << 16) | u32::from(z_south), // SE
    ]
}

pub struct StaticQueryWorld {
    bodies: RigidBodySet,
    colliders: ColliderSet,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
}

impl StaticQueryWorld {
    pub fn as_query_pipeline<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
        self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        )
    }
}

pub fn build_static_query_world(
    world_statics: impl IntoIterator<Item = WorldStaticDef>,
    dt: f32,
) -> StaticQueryWorld {
    let bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut modified_colliders = Vec::new();

    world_statics.into_iter().for_each(|def| {
        let mut collider = collider_from_def(&def);
        let iso = Isometry::from_parts(Translation3::from(def.translation), def.rotation);
        collider.set_position(iso);
        let co_handle = colliders.insert(collider);
        modified_colliders.push(co_handle);
    });

    let mut broad_phase = BroadPhaseBvh::new();
    let mut events = Vec::new();
    broad_phase.update(
        &IntegrationParameters {
            dt,
            ..IntegrationParameters::default()
        },
        &colliders,
        &bodies,
        &modified_colliders,
        &[],
        &mut events,
    );

    StaticQueryWorld {
        bodies,
        colliders,
        broad_phase,
        narrow_phase: NarrowPhase::default(),
    }
}
