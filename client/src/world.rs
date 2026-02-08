use bevy::prelude::*;
use bevy_spacetimedb::ReadInsertMessage;

use crate::module_bindings::{ColliderShape, WorldStatic};
use shared::{ColliderShapeDef, WorldStaticDef, build_static_query_world};

use rapier3d::na::{UnitQuaternion, Vector3};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ClientStaticQueryWorld>();
    app.add_systems(Startup, setup);
    app.add_systems(Update, load_world);
}

#[derive(Component)]
pub struct Ground;

/// Cached, query-only static collision world built from `world_static` rows.
///
/// For now (until we add incremental updates to `shared`), we rebuild the Rapier
/// query world whenever we receive `WorldStatic` inserts. This matches the server’s
/// logic but happens far less frequently on the client.
///
/// This resource is intended to be used by client-side KCC movement prediction
/// via `StaticQueryWorld::as_query_pipeline(QueryFilter::only_fixed())`.
#[derive(Resource, Default)]
pub struct ClientStaticQueryWorld {
    /// Latest built query world, if available.
    pub query_world: Option<shared::utils::StaticQueryWorld>,
    /// Cached defs collected from server rows.
    defs: Vec<WorldStaticDef>,
}

fn setup(mut commands: Commands) {
    // light
    commands.spawn((
        DirectionalLight {
            illuminance: 80_000.0,
            shadows_enabled: true,
            ..default()
        },
        // Orientation: Looking down from the sky
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::new(1.0, -1.0, 1.0), Vec3::Y),
    ));
}

fn load_world(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<WorldStatic>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query_world: ResMut<ClientStaticQueryWorld>,
) {
    let mut received_any = false;

    for msg in msgs.read() {
        received_any = true;
        let world_static = msg.row.clone();

        // 1) Spawn visuals (existing behavior).
        //
        // Clone the shape so we can also use `world_static` afterwards (e.g. to build collision defs)
        // without partially moving fields out of it.
        let shape = world_static.shape.clone();
        match shape {
            ColliderShape::Plane(_) => {
                commands.spawn((
                    Ground,
                    Pickable::default(),
                    Transform {
                        rotation: world_static.rotation.clone().into(),
                        translation: world_static.translation.clone().into(),
                        scale: world_static.scale.clone().into(),
                    },
                    Mesh3d(
                        meshes.add(
                            Plane3d::default()
                                .mesh()
                                .size(world_static.scale.x, world_static.scale.z)
                                .build(),
                        ),
                    ),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::linear_rgb(0.2, 0.3, 0.25),
                        perceptual_roughness: 1.0,
                        metallic: 0.0,
                        ..default()
                    })),
                ));
            }
            ColliderShape::Cuboid(val) => {
                commands.spawn((
                    Pickable::default(),
                    Transform {
                        rotation: world_static.rotation.clone().into(),
                        translation: world_static.translation.clone().into(),
                        scale: world_static.scale.clone().into(),
                    },
                    // `val` in schema is treated as half-extents (matches server/shared)
                    Mesh3d(meshes.add(Cuboid::new(val.x * 2., val.y * 2., val.z * 2.))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::linear_rgb(0.8, 0.1, 0.15),
                        perceptual_roughness: 1.0,
                        metallic: 0.0,
                        ..default()
                    })),
                ));
            }
            _ => {
                // Keep visuals strict so you notice missing shape implementations early.
                unimplemented!("WorldStatic shape not yet implemented for client visuals")
            }
        }

        // 2) Convert replicated `WorldStatic` into shared `WorldStaticDef` for collision queries.
        if let Some(def) = world_static_to_def(&world_static) {
            query_world.defs.push(def);
        }
    }

    // Any time we receive `world_static` inserts, rebuild the cached query world.
    //
    // NOTE: This is “rebuild-on-insert”. If you later add updates/deletes to `world_static`,
    // you’ll want to rebuild on those too and/or implement incremental updates.
    if received_any {
        // dt is only used for broadphase update params in shared::build_static_query_world.
        // Using 60 Hz is fine for query-only world on the client.
        let dt = 1.0 / 60.0;
        query_world.query_world = Some(build_static_query_world(query_world.defs.clone(), dt));
    }
}

fn world_static_to_def(row: &WorldStatic) -> Option<WorldStaticDef> {
    let translation = Vector3::new(row.translation.x, row.translation.y, row.translation.z);

    // Convert schema quaternion to nalgebra UnitQuaternion.
    //
    // SpacetimeDB binding stores (x,y,z,w); nalgebra expects (w, i, j, k) when using Quaternion::new.
    // UnitQuaternion::from_quaternion will normalize.
    let quat = rapier3d::na::Quaternion::new(
        row.rotation.w,
        row.rotation.x,
        row.rotation.y,
        row.rotation.z,
    );
    let rotation = UnitQuaternion::from_quaternion(quat);

    let shape = match &row.shape {
        ColliderShape::Plane(offset_along_normal) => ColliderShapeDef::Plane {
            offset_along_normal: *offset_along_normal,
        },
        ColliderShape::Cuboid(he) => ColliderShapeDef::Cuboid {
            half_extents: Vector3::new(he.x, he.y, he.z),
        },
        ColliderShape::Sphere(r) => ColliderShapeDef::Sphere { radius: *r },
        ColliderShape::CapsuleY(c) => ColliderShapeDef::CapsuleY {
            radius: c.radius,
            half_height: c.half_height,
        },
        ColliderShape::Cylinder(c) => ColliderShapeDef::CylinderY {
            radius: c.radius,
            half_height: c.half_height,
        },
        ColliderShape::Cone(c) => ColliderShapeDef::ConeY {
            radius: c.radius,
            half_height: c.half_height,
        },
        ColliderShape::RoundCuboid(rc) => ColliderShapeDef::RoundCuboid {
            half_extents: Vector3::new(rc.half_extents.x, rc.half_extents.y, rc.half_extents.z),
            border_radius: rc.border_radius,
        },
        ColliderShape::RoundCylinder(rc) => ColliderShapeDef::RoundCylinderY {
            radius: rc.radius,
            half_height: rc.half_height,
            border_radius: rc.border_radius,
        },
        ColliderShape::RoundCone(rc) => ColliderShapeDef::RoundConeY {
            radius: rc.radius,
            half_height: rc.half_height,
            border_radius: rc.border_radius,
        },
    };

    Some(WorldStaticDef {
        id: row.id,
        translation,
        rotation,
        shape,
    })
}
