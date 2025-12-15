use crate::{
    schema::{world_static, WorldStatic},
    types::*,
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
    WorldStaticDef {
        id: row.id,
        translation: row.translation.into(),
        rotation: row.rotation.into(),
        shape: match row.shape {
            ColliderShape::Plane(offset_along_normal) => ColliderShapeDef::Plane {
                offset_along_normal,
            },
            ColliderShape::Cuboid(half_extents) => ColliderShapeDef::Cuboid {
                half_extents: half_extents.into(),
            },
            ColliderShape::Sphere(radius) => ColliderShapeDef::Sphere { radius },
            ColliderShape::Capsule(dim) => ColliderShapeDef::CapsuleY {
                radius: dim.radius,
                half_height: dim.half_height,
            },
            ColliderShape::Cylinder(DbCylinder {
                radius,
                half_height,
            }) => ColliderShapeDef::CylinderY {
                radius,
                half_height,
            },
            ColliderShape::Cone(DbCone {
                radius,
                half_height,
            }) => ColliderShapeDef::ConeY {
                radius,
                half_height,
            },
            ColliderShape::RoundCuboid(DbRoundCuboid {
                half_extents,
                border_radius,
            }) => ColliderShapeDef::RoundCuboid {
                half_extents: half_extents.into(),
                border_radius,
            },
            ColliderShape::RoundCylinder(DbRoundCylinder {
                radius,
                half_height,
                border_radius,
            }) => ColliderShapeDef::RoundCylinderY {
                radius,
                half_height,
                border_radius,
            },
            ColliderShape::RoundCone(DbRoundCone {
                radius,
                half_height,
                border_radius,
            }) => ColliderShapeDef::RoundConeY {
                radius,
                half_height,
                border_radius,
            },
        },
    }
}
