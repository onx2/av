//! World geometry loader and Rapier query-world cache.
//!
//! This module is responsible for:
//! - Reading immutable world-static collider rows from the database.
//! - Converting them to shared, schema-agnostic collider definitions.
//! - Building and caching an in-memory Rapier query world for scene queries and KCC.
//!
//! Design notes
//! - World statics are treated as immutable. We build once and reuse every tick.
//! - Determinism: we sort collider definitions by `id` before building the Rapier world.
//! - This cache is intentionally *query-focused*: we build enough Rapier state to run
//!   scene queries (ray casts, shape casts) and the built-in KCC (`KinematicCharacterController`).
//! - This module does NOT step a physics simulation; it just prepares the collision/query structures.
//!
//! Usage
//! - Call [`world_query_world`] to get a `&'static shared::RapierQueryWorld`.
//!
//! All returned references are `'static` because the data are cached with `OnceLock` and
//! never mutated after initialization. If you need runtime world edits, `OnceLock` is not
//! appropriate; add an explicit rebuild/refresh mechanism.

use crate::{
    model::{unit_quat_from_db, vec3_from_db},
    schema::{world_static, ColliderShape, WorldStatic},
};
use shared::{ColliderShapeDef, RapierQueryWorld, WorldStaticDef};
use spacetimedb::{ReducerContext, Table};
use std::sync::OnceLock;

/// Cached in-memory Rapier query world built from immutable `world_static` rows.
static WORLD_QUERY_WORLD: OnceLock<RapierQueryWorld> = OnceLock::new();

/// Return the cached Rapier query world.
///
/// The first call reads the `world_static` table, converts rows to shared definitions,
/// builds the Rapier query world, and caches it. Subsequent calls return the cached world.
///
/// Note: This assumes world statics do not change at runtime.
pub fn world_query_world(ctx: &ReducerContext) -> &'static RapierQueryWorld {
    WORLD_QUERY_WORLD.get_or_init(|| build_world_query_world(ctx))
}

/// Build the Rapier query world from DB rows (`world_static`).
///
/// Mapping rules:
/// - Plane(offset): uses `normal = rotation * +Y` and `dist = dot(normal, translation) + offset`.
/// - Cuboid(half_extents): oriented box using the row's translation+rotation.
/// - Capsule(dim): Y-aligned capsule using the row's translation+rotation.
///
/// Scale:
/// - This builder intentionally ignores scaling (assumes scale is 1) for determinism and simplicity.
///   If your DB schema still contains scale fields, they are ignored here.
fn build_world_query_world(ctx: &ReducerContext) -> RapierQueryWorld {
    let defs = ctx.db.world_static().iter().map(row_to_def).collect();
    RapierQueryWorld::build(defs)
}

/// Convert a single `WorldStatic` row to the shared schema-agnostic definition.
fn row_to_def(row: WorldStatic) -> WorldStaticDef {
    // Convert from DB types to nalgebra types.
    // These helpers already exist in your codebase and are used elsewhere.
    let t = vec3_from_db(row.translation);
    let q = unit_quat_from_db(row.rotation);

    // Shared Rapier-world defs use rapier3d/nalgebra vector + unit quaternion types.
    // The helper outputs should already be compatible (`nalgebra`), and rapier3d re-exports nalgebra.
    // We rely on the fact both are the same nalgebra types through workspace settings.
    let translation = shared::rapier_world::rapier3d::prelude::vector![t.x, t.y, t.z];

    let rotation = q;

    let shape = match row.shape {
        ColliderShape::Plane(offset_along_normal) => ColliderShapeDef::Plane {
            offset_along_normal,
        },
        ColliderShape::Cuboid(he) => {
            let he = vec3_from_db(he);
            ColliderShapeDef::Cuboid {
                half_extents: shared::rapier_world::rapier3d::prelude::vector![he.x, he.y, he.z],
            }
        }
        ColliderShape::Capsule(dim) => ColliderShapeDef::CapsuleY {
            radius: dim.radius,
            half_height: dim.half_height,
        },
    };

    WorldStaticDef {
        id: row.id as u64,
        translation,
        rotation,
        shape,
    }
}
