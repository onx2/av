//! Server crate entry point.
//!
//! This module wires together:
//! - The database schema and serialization types.
//! - World geometry loading and broad-phase accelerator caching.
//! - Thin physics helpers around the shared KCC (kinematic character controller).
//! - Reducers split by concern (connection, movement, tick).
//!
//! Organization
//! - schema: All SpacetimeDB tables and tagged-union types (authoritative data).
//! - model:  Conversions between schema types (DbVec3/DbQuat) and nalgebra types, plus small math helpers.
//! - world:  Loads immutable world statics from the DB and caches a broad-phase accelerator (AABB pruning).
//! - physics: Thin wrappers around shared KCC for readable tick pipelines (move → collide → snap → gravity).
//! - reducers:
//!   - connection: create/cleanup Player rows on connect/disconnect.
//!   - movement:   enter/leave world and request movement intent.
//!   - tick:       authoritative per-frame kinematic simulation (sweep-slide + snap).
//!
//! Tick scheduling
//! - This file declares the scheduled `tick` table/row (`TickTimer`) and the `init` reducer,
//!   which configures the server to run the tick at a fixed cadence (e.g., 60 Hz).
use spacetimedb::*;

pub mod model;
pub mod physics;
pub mod schema;
pub mod world;

// Reducers split by concern.
pub mod reducers {
    pub mod connection;
    pub mod movement;
    pub mod tick;
}

// Re-export reducers so callers (and generated bindings) can refer to them at the crate root.
use crate::schema::world_static;
pub use reducers::{
    connection::{identity_connected, identity_disconnected},
    movement::{enter_world, leave_world, request_move},
    tick::tick,
};

/// Target tick rate and derived fallback delta for defensive timing.
///
/// The reducer uses the timestamp difference between the current invocation
/// and the last tick for delta time. In the unlikely event the delta cannot
/// be computed, it falls back to this target cadence.
const TICK_RATE: i64 = 60;
const DELTA_MICRO_SECS: i64 = 1_000_000 / TICK_RATE;

/// Scheduled tick row used by SpacetimeDB to invoke the authoritative `tick` reducer.
///
/// The server inserts a single row with a fixed interval in `init` and updates the
/// `last_tick` timestamp on every invocation to derive `delta_time_seconds` deterministically.
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
}

/// Module initialization.
///
/// Responsibilities:
/// - Configure the scheduled `tick` reducer to run at a fixed cadence.
/// - (Optional) Seed example world statics if you want a default scene to exist.
///
/// Determinism:
/// - This reducer only performs data-layer operations (no physics).
#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Configure the scheduled tick with a fixed interval.
    let tick_interval = TimeDuration::from_micros(DELTA_MICRO_SECS);
    ctx.db.tick_timer().scheduled_id().delete(1);
    ctx.db.tick_timer().insert(TickTimer {
        scheduled_id: 1,
        scheduled_at: spacetimedb::ScheduleAt::Interval(tick_interval),
        last_tick: ctx.timestamp,
    });

    // Optional: seed a simple world if the table is empty.
    use schema::{ColliderShape, DbQuat, DbVec3, WorldStatic};

    if ctx.db.world_static().count() == 0 {
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
    }
}
