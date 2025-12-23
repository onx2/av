use crate::{
    schema::{kcc_settings, world_static, WorldStatic},
    types::*,
};
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use shared::{ColliderShapeDef, RapierQueryWorld, WorldStaticDef};
use spacetimedb::{ReducerContext, Table};
use std::sync::OnceLock;

/// Cached in-memory Rapier query world built from immutable `world_static` rows.
static SHARED_KCC: OnceLock<KinematicCharacterController> = OnceLock::new();

/// Return the cached Rapier query world.
///
/// The first call reads the `world_static` table, converts rows to shared definitions,
/// builds the Rapier query world, and caches it. Subsequent calls return the cached world.
///
/// Note: This assumes world statics do not change at runtime.
pub fn get_kcc(ctx: &ReducerContext) -> &'static KinematicCharacterController {
    SHARED_KCC.get_or_init(|| {
        let kcc = ctx
            .db
            .kcc_settings()
            .id()
            .find(1)
            .expect("Missing kcc_settings row (expected id = 1)");

        KinematicCharacterController {
            offset: CharacterLength::Absolute(kcc.offset),
            max_slope_climb_angle: kcc.max_slope_climb_deg.to_radians(),
            min_slope_slide_angle: kcc.min_slope_slide_deg.to_radians(),
            snap_to_ground: Some(CharacterLength::Absolute(0.1)),
            autostep: Some(CharacterAutostep {
                max_height: CharacterLength::Absolute(kcc.autostep_max_height),
                min_width: CharacterLength::Absolute(kcc.autostep_min_width),
                include_dynamic_bodies: false,
            }),
            slide: kcc.slide,
            normal_nudge_factor: kcc.normal_nudge_factor,
            ..KinematicCharacterController::default()
        }
    })
}

/// Cached in-memory Rapier query world built from immutable `world_static` rows.
static WORLD_QUERY_WORLD: OnceLock<RapierQueryWorld> = OnceLock::new();

/// Return the cached Rapier query world.
///
/// The first call reads the `world_static` table, converts rows to shared definitions,
/// builds the Rapier query world, and caches it. Subsequent calls return the cached world.
///
/// Note: This assumes world statics do not change at runtime.
pub fn get_rapier_world(ctx: &ReducerContext) -> &'static RapierQueryWorld {
    WORLD_QUERY_WORLD.get_or_init(|| {
        let defs = ctx.db.world_static().iter().map(row_to_def).collect();
        RapierQueryWorld::build(defs)
    })
}

/// Convert a single `WorldStatic` row to the shared schema-agnostic definition.
fn row_to_def(row: WorldStatic) -> WorldStaticDef {
    let shape = match row.shape {
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
    };

    WorldStaticDef {
        id: row.id,
        translation: row.translation.into(),
        rotation: row.rotation.into(),
        shape,
    }
}
