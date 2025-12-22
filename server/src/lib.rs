pub mod schema;
pub mod types;
pub mod utils;
pub mod world;

// Reducers split by concern.
pub mod reducers {
    // pub mod aoi_tick;
    pub mod connection;
    pub mod enter_world;
    pub mod leave_world;
    pub mod movement_tick;
    pub mod request_move;

    // Periodic corrective nudges (e.g. push idle overlapping players apart).
    pub mod overlap_push;

    // Temporary dev tooling: spawn and drive fake (non-player) actors.
    pub mod spawn_fake;
}

// Re-export reducers so callers (and generated bindings) can refer to them at the crate root.
use crate::{
    reducers::{movement_tick, overlap_push, spawn_fake},
    schema::*,
    types::*,
};
use spacetimedb::*;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Configure scheduled ticks:
    // - player movement @ 30 Hz
    // - non-player movement @ 15 Hz
    movement_tick::init(ctx);
    // aoi_tick::init(ctx);

    // Start the idle overlap push tick (10Hz by default).
    overlap_push::init(ctx);

    // Start the scheduled fake wandering driver (no-op if no fakes exist).
    spawn_fake::init(ctx);

    // Seed default KCC settings (single-row table).
    // This allows both server and clients to use the exact same tuning parameters.
    ctx.db.kcc_settings().id().delete(1);
    ctx.db.kcc_settings().insert(KccSettings {
        id: 1,
        offset: 0.05,
        max_slope_climb_deg: 52.0,
        min_slope_slide_deg: 45.0,
        autostep_max_height: 0.325,
        autostep_min_width: 0.2,
        slide: true,
        normal_nudge_factor: 0.05,
        fall_speed_mps: 9.82,
        grounded_down_bias_mps: 0.5,
        hard_airborne_probe_distance: 0.6,
        point_acceptance_radius_sq: 0.0225,
    });

    for row in ctx.db.world_static().iter() {
        ctx.db.world_static().delete(row);
    }

    // Infinite ground plane at y = 0.
    ctx.db.world_static().insert(WorldStatic {
        id: 0,
        translation: DbVec3::new(0.0, 0.0, 0.0),
        rotation: DbQuat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        // Visual-only for planes.
        scale: DbVec3::new(10.0, 1.0, 10.0),
        shape: ColliderShape::Plane(0.0),
    });

    // A simple oriented cuboid test object.
    ctx.db.world_static().insert(WorldStatic {
        id: 0,
        translation: DbVec3::new(3.0, 1.0, 0.0),
        rotation: DbQuat {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            w: 0.0,
        },
        scale: DbVec3::new(1.0, 1.0, 1.0),
        // Half-extents (hx, hy, hz) before scale is applied by the server's world loader.
        shape: ColliderShape::Cuboid(DbVec3::new(1.0, 1.0, 1.0)),
    });

    // A downhill ramp (tilted cuboid) to test snap-to-ground and slope behavior.
    // Tilt around X by -20 degrees so moving +Z goes "uphill".
    ctx.db.world_static().insert(WorldStatic {
        id: 0,
        translation: DbVec3::new(-3.0, 0.0, 6.0),
        rotation: DbQuat {
            x: -0.17364818,
            y: 0.0,
            z: 0.0,
            w: 0.98480775,
        },
        scale: DbVec3::new(1.0, 1.0, 1.0),
        shape: ColliderShape::Cuboid(DbVec3::new(1.0, 1.0, 10.0)),
    });

    // A simple staircase to test autostep up/down.
    let stairs_origin = DbVec3::new(0.0, 0.0, -6.0);
    let step_run: f32 = 0.55;
    let step_rise: f32 = 0.4;
    let step_count: u32 = 20;

    // Half extents for each step: total height = 0.20, total depth = 0.50.
    let step_half = DbVec3::new(step_run * 0.5, step_rise * 0.5, 1.5);

    for i in 0..step_count {
        let ix = i as u32;
        let fx = ix as f32;

        // Center of the step in world space.
        let cx = stairs_origin.x + fx * step_run;
        let cy = stairs_origin.y + (fx * step_rise) + step_half.y;
        let cz = stairs_origin.z;

        ctx.db.world_static().insert(WorldStatic {
            id: 0,
            translation: DbVec3::new(cx, cy, cz),
            rotation: DbQuat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            scale: DbVec3::ONE,
            shape: ColliderShape::Cuboid(step_half),
        });
    }
}
