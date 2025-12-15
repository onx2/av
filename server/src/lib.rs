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
use crate::schema::world_static;
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

    // Optional: seed a simple world if the table is empty.
    use schema::{ColliderShape, DbQuat, DbVec3, WorldStatic};

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
            z: 0.0,
            w: 1.0,
        },
        scale: DbVec3::new(1.0, 1.0, 1.0),
        // Half-extents (hx, hy, hz) before scale is applied by the server's world loader.
        shape: ColliderShape::Cuboid(DbVec3::new(0.1, 1.0, 2.0)),
    });

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
}
