use crate::{WorldStaticDef, collider_from_def};

use super::constants::*;
use nalgebra::{self as na, Isometry, Translation3, point, vector};
use rapier3d::prelude::{
    BroadPhaseBvh, ColliderSet, IntegrationParameters, NarrowPhase, QueryFilter, QueryPipeline,
    Ray, RigidBodySet,
};
use std::f32::consts::TAU;

pub fn yaw_from_xz(xz: &na::Vector2<f32>) -> Option<f32> {
    if xz.norm_squared() > YAW_EPS {
        return Some((-xz[0]).atan2(-xz[1]));
    }

    None
}

/// Returns true if two world positions are within the planar (XZ) acceptance radius.
pub fn is_at_target_planar(
    current: [f32; 2],
    target: [f32; 2],
    point_acceptance_radius_sq: f32,
) -> bool {
    let dx = target[0] - current[0];
    let dz = target[1] - current[1];
    (dx * dx) + (dz * dz) <= point_acceptance_radius_sq
}

/// Computes the desired_translation used as an input to KCC's move_shape function.
pub fn compute_desired_translation(
    current_planar: [f32; 2],
    target_planar: [f32; 2],
    movement_speed_mps: f32,
    dt: f32,
    supported: bool,
    grounded_down_bias_mps: f32,
    fall_speed_mps: f32,
    point_acceptance_radius_sq: f32,
) -> [f32; 3] {
    // Planar displacement (XZ)
    let current_planar = na::Vector2::new(current_planar[0], current_planar[1]);
    let target_planar = na::Vector2::new(target_planar[0], target_planar[1]);

    let max_step = movement_speed_mps * dt;
    let displacement = target_planar - current_planar;
    let dist_sq = displacement.norm_squared();

    // If we're already at the target (within the same acceptance radius used by movement intent),
    // we still apply vertical bias/gravity, but no planar movement.
    let mut desired_planar = if dist_sq <= point_acceptance_radius_sq {
        na::Vector2::new(0.0, 0.0)
    } else {
        let dist = dist_sq.sqrt();
        displacement * (max_step.min(dist) / dist)
    };

    // Vertical components
    let down_bias = -grounded_down_bias_mps * dt;
    let gravity = if supported {
        0.0
    } else {
        desired_planar *= 0.35;
        -fall_speed_mps * dt
    };

    [desired_planar.x, down_bias + gravity, desired_planar.y]
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

pub fn has_support_within(
    query_pipeline: &QueryPipeline<'_>,
    translation: &[f32; 3],
    capsule_half_height: f32,
    capsule_radius: f32,
    max_dist: f32,
    min_ground_normal_y: f32,
) -> bool {
    // Probe from the capsule "feet" (slightly above to avoid starting inside geometry).
    let feet_y: f32 = translation[1] - (capsule_half_height + capsule_radius);
    let origin_y = feet_y + 0.02;

    let ray = Ray::new(
        point![translation[0], origin_y, translation[2]],
        vector![0.0, -1.0, 0.0],
    );

    if let Some((_handle, hit)) =
        query_pipeline.cast_ray_and_get_normal(&ray, max_dist.max(0.0), true)
    {
        hit.normal.y >= min_ground_normal_y
    } else {
        false
    }
}

/// Owns the Rapier structures needed to create a `QueryPipeline<'_>` in Rapier 0.31.
///
/// In 0.31, `QueryPipeline` borrows the broad-phase BVH and the sets, so you can't
/// return it directly from a builder function without also returning the owned data.
pub struct StaticQueryWorld {
    bodies: RigidBodySet,
    colliders: ColliderSet,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
}

impl StaticQueryWorld {
    pub fn as_query_pipeline(&self) -> QueryPipeline<'_> {
        self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            QueryFilter::default(),
        )
    }
}

pub fn build_static_query_world(world_statics: Vec<WorldStaticDef>, dt: f32) -> StaticQueryWorld {
    let bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut modified_colliders = Vec::new();

    world_statics.iter().for_each(|def| {
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
