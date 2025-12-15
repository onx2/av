use spacetimedb::*;

pub mod model;
pub mod schema;
pub mod world;

// Reducers split by concern.
pub mod reducers {
    pub mod connection;
    pub mod enter_world;
    pub mod leave_world;
    pub mod movement;
    pub mod tick;
}

// Re-export reducers so callers (and generated bindings) can refer to them at the crate root.
use crate::schema::{
    kcc_settings, world_static, ColliderShape, DbQuat, DbVec3, KccSettings, WorldStatic,
};
pub use reducers::{
    connection::{identity_connected, identity_disconnected},
    enter_world, leave_world,
    movement::request_move,
    tick::tick,
};

const TICK_RATE: i64 = 60;
const DELTA_MICRO_SECS: i64 = 1_000_000 / TICK_RATE;

#[table(name = tick_timer, scheduled(tick))]
pub struct TickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: spacetimedb::ScheduleAt,
    /// Timestamp of the previous invocation (authoritative delta time source).
    pub last_tick: Timestamp,
    /// The time deficit left over from the last run.
    pub time_accumulator: f32,
}

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Configure the scheduled tick with a fixed interval.
    let tick_interval = TimeDuration::from_micros(DELTA_MICRO_SECS);
    ctx.db.tick_timer().scheduled_id().delete(1);
    ctx.db.tick_timer().insert(TickTimer {
        scheduled_id: 1,
        scheduled_at: spacetimedb::ScheduleAt::Interval(tick_interval),
        last_tick: ctx.timestamp,
        time_accumulator: 0.,
    });

    // Seed default KCC settings (single-row table).
    // This allows both server and clients to use the exact same tuning parameters.
    ctx.db.kcc_settings().id().delete(1);
    ctx.db.kcc_settings().insert(KccSettings {
        id: 1,

        // KCC lengths (ABSOLUTE METERS):
        // Tick configures Rapier with `CharacterLength::Absolute` for offset/autostep/snap-to-ground,
        // so these values must be interpreted as meters (not relative fractions).
        //
        // For reference, your current character capsule is:
        // - radius = 0.35m (diameter = 0.70m)
        // - half_height = 0.75m (total height ≈ 2.20m)
        //
        // Keep `offset` small but non-zero (numerical stability).
        offset: 0.02,

        // Slopes (degrees; tick converts to radians).
        max_slope_climb_deg: 52.0,
        min_slope_slide_deg: 35.0,

        // Snap-to-ground distance (meters).
        snap_to_ground: 1.0,

        // Autostep (meters).
        autostep_max_height: 1.30,
        autostep_min_width: 0.30,

        // Controller behavior.
        slide: true,

        // Increase slightly if the character gets stuck on edges when sliding.
        normal_nudge_factor: 0.05,

        // Movement glue values (used by tick):
        // Constant fall speed and a slight downward bias to satisfy snap-to-ground prerequisites.
        fall_speed_mps: 9.82,
        grounded_down_bias_mps: 0.5,
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
    //
    // Each "step" is a short cuboid. With:
    // - autostep_max_height = 0.35
    // - autostep_min_width  = 0.20
    // the step rise must be <= 0.35 and the flat depth should be >= 0.20.
    //
    // We'll build 6 steps going in +X with:
    // - rise = 0.20m per step
    // - run  = 0.50m per step
    //
    // The cuboid is centered at its translation, so y = (height/2) + step_index * rise.
    // We'll keep width (Z) fairly wide so it's easy to walk on.
    let stairs_origin = DbVec3::new(0.0, 0.0, -6.0);
    let step_run: f32 = 0.85;
    let step_rise: f32 = 0.25;
    let step_count: u32 = 10;

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
            scale: DbVec3::new(1.0, 1.0, 1.0),
            shape: ColliderShape::Cuboid(step_half),
        });
    }
}
