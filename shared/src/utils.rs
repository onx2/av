use crate::{
    GRAVITY_MPS2, MAX_INTENT_DISTANCE_SQ, SMALLEST_REQUEST_DISTANCE_SQ, TERMINAL_FALL_SPEED_MPS,
    WorldStaticDef, YAW_EPS, collider_from_def, dequantize_vertical_velocity,
    quantize_vertical_velocity,
};
use nalgebra::{Isometry, Translation3, Vector2, Vector3};
use rapier3d::prelude::{
    BroadPhaseBvh, ColliderSet, IntegrationParameters, NarrowPhase, QueryFilter, QueryPipeline,
    RigidBodySet,
};
use std::f32::consts::TAU;

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

// /// Quantize radians into a u16 [0, 65535].
// pub fn yaw_to_u16(yaw_radians: f32) -> u16 {
//     const SCALE: f32 = 65536.0 / TAU;

//     // 1. Multiply to get range approx [-32768.0, 32768.0] (if input is -PI to PI)
//     // 2. Cast to i32 to handle negative signs via bit wrapping
//     // 3. Cast to u16 to truncate to the 0..65535 range
//     (yaw_radians * SCALE) as i32 as u16
// }

// /// Dequantize `u16` yaw back into radians in [0, 2π).
// pub fn yaw_from_u16(code: u16) -> f32 {
//     const REV_SCALE: f32 = TAU / 65536.0;

//     (code as f32) * REV_SCALE
// }

/// Returns true if two world positions are within the planar (XZ) acceptance radius.
pub fn is_at_target_planar(current: Vector2<f32>, target: Vector2<f32>) -> bool {
    const CM_SQ: f32 = 1.0e-4;
    (target - current).norm_squared() <= CM_SQ
}

pub fn get_desired_delta(
    current_planar: Vector2<f32>,
    target_planar: Vector2<f32>,
    movement_speed_mps: f32,
    vertical_velocity: i8,
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
        let v_mps = dequantize_vertical_velocity(vertical_velocity);
        // Air control reduction in planar and gravity.
        [
            x * AIR_CONTROL_REDUCTION,
            v_mps * dt,
            z * AIR_CONTROL_REDUCTION,
        ]
        .into()
    }
}

/// Gets the next vertical velocity step while falling
pub fn advance_vertical_velocity(vel_q: i8, dt: f32) -> i8 {
    if vel_q >= 0 {
        return 0;
    }

    let v0_mps = dequantize_vertical_velocity(vel_q);

    // Semi-implicit Euler: v(t+dt) = v(t) + g*dt
    let mut v1_mps = v0_mps + GRAVITY_MPS2 * dt;

    // Clamp to terminal fall speed (negative/downward). We only do this when already falling.
    if v1_mps < TERMINAL_FALL_SPEED_MPS {
        v1_mps = TERMINAL_FALL_SPEED_MPS;
    }

    // Re-quantize to i8.
    quantize_vertical_velocity(v1_mps)
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
