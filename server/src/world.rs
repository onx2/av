use crate::{
    schema::{world_static, WorldStatic},
    types::*,
};
use shared::{ColliderShapeDef, WorldStaticDef};
use spacetimedb::{ReducerContext, Table};

/// Convert a single `WorldStatic` row to the shared schema-agnostic definition.
pub fn row_to_def(row: WorldStatic) -> WorldStaticDef {
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

pub fn recreate_static_world(ctx: &ReducerContext) {
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
