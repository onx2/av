use crate::{WorldStaticDef, collider_from_def};

use super::constants::*;
use nalgebra::{self as na, Isometry, Translation3, Vector2, Vector3};
use rapier3d::prelude::{
    BroadPhaseBvh, ColliderSet, IntegrationParameters, NarrowPhase, QueryFilter, QueryPipeline,
    RigidBodySet,
};
use std::f32::consts::TAU;

pub fn yaw_from_xz(xz: [f32; 2]) -> Option<f32> {
    let xz: na::Vector2<f32> = xz.into();
    if xz.norm_squared() > YAW_EPS {
        return Some((-xz[0]).atan2(-xz[1]));
    }

    None
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
    grounded: bool,
    dt: f32,
) -> Vector3<f32> {
    const MM_SQ: f32 = 1.0e-6;

    let max_step = movement_speed_mps * dt;
    let displacement = target_planar - current_planar;
    let dist_sq = displacement.norm_squared();

    let desired_planar = if dist_sq <= MM_SQ {
        na::Vector2::new(0.0, 0.0)
    } else {
        let dist = dist_sq.sqrt();
        displacement * (max_step.min(dist) / dist)
    };

    if grounded {
        // No need for downward bias or gravity because snap to ground is active
        [desired_planar.x, 0.0, desired_planar.y].into()
    } else {
        // Air control reduction in planar and gravity.
        // Gravity is linear because I don't expect to have a need for "real" gravity...
        // in the world, it is only applied here to make sure we end up on the ground eventually.
        [desired_planar.x * 0.35, -9.81 * dt, desired_planar.y * 0.35].into()
    }
}

/// Quantize yaw (radians) into a 2 bytes.
///
/// Convention:
/// - input: yaw in radians (any range; e.g. [-π, π] or [0, 2π))
/// - output: `u8` in 0..=255 representing [0, 2π) in 256 uniform steps
pub fn yaw_to_u16(yaw_radians: f32) -> u16 {
    // 65536.0 / 2π
    const SCALE: f32 = 65536.0 / TAU;

    // We use rem_euclid to ensure the angle is in the [0, TAU) range
    // before scaling. This handles negative radians correctly.
    let normalized_yaw = yaw_radians.rem_euclid(TAU);

    // Multiply by scale and truncate.
    // Since normalized_yaw < TAU, (yaw * SCALE) will be < 65536.0
    (normalized_yaw * SCALE) as u32 as u16
}

/// Dequantize `u16` yaw back into radians in [0, 2π).
pub fn yaw_from_u16(code: u16) -> f32 {
    // 2π / 65536.0
    const INV_SCALE: f32 = TAU / 65536.0;

    (code as f32) * INV_SCALE
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
pub fn get_aoi_block(cell_id: u32) -> [u32; 9] {
    // Cell ID format: [x: u16 (bits 31-16)] [z: u16 (bits 15-0)]
    let x = (cell_id >> 16) as u16; // Extract grid X from low 16 bits after right shift
    let z = (cell_id & 0xFFFF) as u16; // Extract grid Z from low 16 bits after bitwise AND

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
        cell_id,                    // Center
        xe_shifted | u32::from(z),  // E
        xw_shifted | u32::from(zs), // SW
        x_shifted | u32::from(zs),  // S
        xe_shifted | u32::from(zs), // SE
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
