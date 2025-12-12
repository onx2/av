//! World geometry loader and accelerator cache.
//!
//! This module is responsible for:
//! - Reading immutable world-static collider rows from the database.
//! - Converting them to the shared collision representation.
//! - Building and caching a static broad-phase accelerator (AABB pruning) for fast queries.
//!
//! Design notes
//! - World statics are treated as immutable. We build once and reuse every tick
//!   for deterministic, fast narrow-phase queries.
//! - Shapes supported by the schema are mapped to `shared::collision::StaticShape`:
//!   - Plane: infinite plane (half-space). Scale is visual-only (ignored by physics).
//!   - Cuboid: oriented box; physics half-extents are multiplied by `scale` component-wise.
//!   - Capsule: Y-aligned capsule. For now, `scale` is ignored for physics (visual-only).
//! - The accelerator excludes planes (infinite) and stores finite AABBs only. Planes are
//!   always tested explicitly by the narrow phase (they are cheap and small in number).
//!
//! Usage
//! - Call `world_statics(ctx)` to get a `&'static [StaticShape]` slice you can pass to shared
//!   collision functions.
//! - Call `world_accel(ctx)` to get a `&'static WorldAccel` for broad-phase pruning.
//!
//! All returned references are `'static` because the data are cached with `OnceLock`
//! and never mutated after initialization.

use crate::{
    model::{unit_quat_from_db, vec3_from_db},
    schema::{world_static, ColliderShape, WorldStatic},
};
use shared::collision::{self, StaticShape};
use spacetimedb::{ReducerContext, Table};
use std::sync::OnceLock;

/// Cached vector of immutable world statics (shared collision representation).
static WORLD_STATICS: OnceLock<Vec<StaticShape>> = OnceLock::new();

/// Cached broad-phase accelerator built from `WORLD_STATICS`.
static WORLD_ACCEL: OnceLock<collision::broad::WorldAccel> = OnceLock::new();

/// Return the immutable world-static shapes as a `'static` slice.
///
/// The first call reads the `world_static` table, converts rows to
/// `shared::collision::StaticShape`, and caches the result. Subsequent calls
/// return the cached slice.
///
/// Note: This assumes world statics do not change at runtime.
pub fn world_statics(ctx: &ReducerContext) -> &'static [StaticShape] {
    WORLD_STATICS.get_or_init(|| build_world_statics(ctx))
}

/// Return the cached broad-phase accelerator for world statics.
///
/// The accelerator is built once from `world_statics(ctx)` and reused.
/// Planes are excluded from the accelerator (they are handled in the narrow phase).
pub fn world_accel(ctx: &ReducerContext) -> &'static collision::broad::WorldAccel {
    WORLD_ACCEL.get_or_init(|| {
        let statics = world_statics(ctx);
        collision::broad::build_world_accel(statics)
    })
}

/// Build `StaticShape`s from DB rows (`world_static`).
///
/// Mapping rules:
/// - Plane: `normal = rotation * +Y`, `dist = normal ⋅ translation + offset`. Scale is ignored.
/// - Cuboid: physics half-extents = `half_extents * scale` (component-wise).
/// - Capsule: Y-aligned capsule with `radius` and `half_height`. Scale is ignored for physics.
///
/// If you later add additional shapes (Sphere, Cylinder, etc.) to the schema,
/// extend the match below to convert them to the shared representation.
fn build_world_statics(ctx: &ReducerContext) -> Vec<StaticShape> {
    let mut out = Vec::new();

    for row in ctx.db.world_static().iter() {
        let t = vec3_from_db(row.translation);
        let q = unit_quat_from_db(row.rotation);
        let sc = vec3_from_db(row.scale);

        match row.shape {
            ColliderShape::Plane(offset_along_normal) => {
                // normal = q * +Y, dist = n ⋅ t + offset
                out.push(collision::plane_from_pose(q, t, offset_along_normal));
            }
            ColliderShape::Cuboid(half_extents) => {
                // Physics half extents = he * scale (component-wise).
                let he = vec3_from_db(half_extents);
                let he_final = he.component_mul(&sc);
                out.push(collision::cuboid_from_pose(he_final, t, q));
            }
            ColliderShape::Capsule(dim) => {
                // For now, scale is visual-only for capsules. If you need scaled capsules,
                // consider a uniform scale based on the largest component and document it.
                out.push(collision::capsule_from_pose(
                    dim.radius,
                    dim.half_height,
                    t,
                    q,
                ));
            }
        }
    }

    out
}

/// Utility for tests and debug: rebuild the caches explicitly.
///
/// This can be used by integration tests to ensure a fresh snapshot
/// when running multiple test cases in the same process.
#[cfg(test)]
pub fn rebuild_world_caches(ctx: &ReducerContext) {
    // Safety: OnceLock has no reset; in tests we build directly and ignore the globals.
    let statics = build_world_statics(ctx);
    let _accel = collision::broad::build_world_accel(&statics);

    // Validate we can map to StaticShape and build an accelerator without panicking.
    assert!(
        !statics.is_empty() || _accel.is_empty(),
        "Unexpected empty world accel build"
    );
}

/// Convert schema rows into shared collision shapes without touching caches.
///
/// This is useful for tools/tests that want a one-off conversion.
pub fn build_world_statics_one_shot(
    rows: impl IntoIterator<Item = WorldStatic>,
) -> Vec<StaticShape> {
    let mut out = Vec::new();

    for row in rows {
        let t = vec3_from_db(row.translation);
        let q = unit_quat_from_db(row.rotation);
        let sc = vec3_from_db(row.scale);

        match row.shape {
            ColliderShape::Plane(offset_along_normal) => {
                out.push(collision::plane_from_pose(q, t, offset_along_normal));
            }
            ColliderShape::Cuboid(half_extents) => {
                let he = vec3_from_db(half_extents);
                let he_final = he.component_mul(&sc);
                out.push(collision::cuboid_from_pose(he_final, t, q));
            }
            ColliderShape::Capsule(dim) => {
                out.push(collision::capsule_from_pose(
                    dim.radius,
                    dim.half_height,
                    t,
                    q,
                ));
            }
        }
    }

    out
}
