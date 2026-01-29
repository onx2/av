use crate::{ColliderShape, Cone, Cylinder, Quat, RoundCone, RoundCuboid, RoundCylinder, Vec3};
use shared::{ColliderShapeDef, WorldStaticDef};
use spacetimedb::{table, ReducerContext, Table};

/// Static collider rows used to build the immutable world collision geometry.
///
/// The server reads these rows into an in-memory Rapier query world for use in
/// scene queries and the kinematic character controller (KCC).
#[table(name = world_static_tbl, public)]
pub struct WorldStatic {
    /// Unique id (primary key).
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    /// World transform applied to the shape.
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    /// Collider shape definition.
    pub shape: ColliderShape,
}
impl WorldStatic {
    pub fn insert(ctx: &ReducerContext, ws: WorldStatic) -> Self {
        ctx.db.world_static_tbl().insert(ws)
    }
    pub fn clear(ctx: &ReducerContext) {
        for row in ctx.db.world_static_tbl().iter() {
            ctx.db.world_static_tbl().delete(row);
        }
    }
}

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
        ColliderShape::Cylinder(Cylinder {
            radius,
            half_height,
        }) => ColliderShapeDef::CylinderY {
            radius,
            half_height,
        },
        ColliderShape::Cone(Cone {
            radius,
            half_height,
        }) => ColliderShapeDef::ConeY {
            radius,
            half_height,
        },
        ColliderShape::RoundCuboid(RoundCuboid {
            half_extents,
            border_radius,
        }) => ColliderShapeDef::RoundCuboid {
            half_extents: half_extents.into(),
            border_radius,
        },
        ColliderShape::RoundCylinder(RoundCylinder {
            radius,
            half_height,
            border_radius,
        }) => ColliderShapeDef::RoundCylinderY {
            radius,
            half_height,
            border_radius,
        },
        ColliderShape::RoundCone(RoundCone {
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
    for row in ctx.db.world_static_tbl().iter() {
        ctx.db.world_static_tbl().delete(row);
    }

    // Infinite ground plane at y = 0.
    WorldStatic::insert(
        ctx,
        WorldStatic {
            id: 0,
            translation: Vec3::ZERO,
            rotation: Quat {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            // Visual-only for planes.
            scale: Vec3::new(10.0, 1.0, 10.0),
            shape: ColliderShape::Plane(0.0),
        },
    );

    // A simple oriented cuboid test object.
    WorldStatic::insert(
        ctx,
        WorldStatic {
            id: 0,
            translation: Vec3::new(3.0, 1.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            // Half-extents (hx, hy, hz) before scale is applied by the server's world loader.
            shape: ColliderShape::Cuboid(Vec3::ONE),
        },
    );

    // A downhill ramp (tilted cuboid) to test snap-to-ground and slope behavior.
    // Tilt around X by -20 degrees so moving +Z goes "uphill".
    WorldStatic::insert(
        ctx,
        WorldStatic {
            id: 0,
            translation: Vec3::new(-3.0, 0.0, 6.0),
            rotation: Quat {
                x: -0.17364818,
                y: 0.0,
                z: 0.0,
                w: 0.98480775,
            },
            scale: Vec3::ONE,
            shape: ColliderShape::Cuboid(Vec3::new(1.0, 1.0, 10.0)),
        },
    );

    // A simple staircase to test autostep up/down.
    let stairs_origin = Vec3::new(0.0, 0.0, -6.0);
    let step_run: f32 = 0.55;
    let step_rise: f32 = 0.4;
    let step_count: u32 = 20;

    // Half extents for each step: total height = 0.20, total depth = 0.50.
    let step_half = Vec3::new(step_run * 0.5, step_rise * 0.5, 1.5);

    for i in 0..step_count {
        let ix = i as u32;
        let fx = ix as f32;

        // Center of the step in world space.
        let cx = stairs_origin.x + fx * step_run;
        let cy = stairs_origin.y + (fx * step_rise) + step_half.y;
        let cz = stairs_origin.z;

        WorldStatic::insert(
            ctx,
            WorldStatic {
                id: 0,
                translation: Vec3::new(cx, cy, cz),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
                shape: ColliderShape::Cuboid(step_half),
            },
        );
    }
}
