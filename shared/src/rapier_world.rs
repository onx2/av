//! Rapier-based query world builder for immutable/static world geometry.
//!
//! This module is intended to be used by both server and client to build an in-memory
//! Rapier scene from a set of static collider definitions (typically sourced from DB rows).
//!
//! Design goals
//! - Deterministic: given the same inputs (sorted by `id`), build identical in-memory sets.
//! - Query-focused: supports scene queries and the Rapier `KinematicCharacterController`.
//! - Immutable world: this builder assumes statics do not move after construction.

// Re-export Rapier so downstream crates (server/client) can use Rapier macros/types
// without needing to depend on `rapier3d` directly.
pub use rapier3d;

use rapier3d::prelude::*;

// Rapier math aliases (0.31): keep these explicit so we don't accidentally rely on
// nalgebra names that aren't in scope.
use rapier3d::na::{Translation3, UnitQuaternion};

/// Canonical, schema-agnostic definition of an immutable world collider.
///
/// Server/client should map DB rows to this type, then call [`RapierQueryWorld::build`].
///
/// Conventions
/// - Units are meters.
/// - Rotation is a unit quaternion.
/// - For planes, we use a pose-derived normal: `normal = rotation * +Y`,
///   and compute `dist = dot(normal, translation) + offset_along_normal`.
#[derive(Clone, Debug)]
pub struct WorldStaticDef {
    /// Stable unique identifier used to ensure deterministic insertion order.
    pub id: u32,
    /// World-space translation.
    pub translation: Vector<f32>,
    /// World-space rotation (unit quaternion).
    pub rotation: UnitQuaternion<f32>,
    /// Collider shape parameters.
    pub shape: ColliderShapeDef,
}

/// Supported static collider shapes.
///
/// Keep this intentionally small and deterministic. Extend as needed.
#[derive(Clone, Debug)]
pub enum ColliderShapeDef {
    /// Infinite plane (half-space).
    ///
    /// This is represented by an offset along the plane normal.
    /// The plane normal is derived from the pose as `rotation * +Y`.
    ///
    /// Notes on Bevy/Rapier "plane size" and rotation
    /// - In Rapier, a plane/half-space is infinite. Any "X/Z size" you see in Bevy is purely a
    ///   rendering/mesh concern, not collision.
    /// - Rotation is fully supported: the half-space normal is derived from the rigid-body pose.
    Plane {
        /// Offset along the plane normal (meters).
        offset_along_normal: f32,
    },

    /// Oriented cuboid with given half-extents (meters).
    Cuboid { half_extents: Vector<f32> },

    /// Sphere/ball (meters).
    Sphere { radius: f32 },

    /// Y-aligned capsule (meters).
    CapsuleY { radius: f32, half_height: f32 },

    /// Y-aligned cylinder (meters).
    CylinderY { radius: f32, half_height: f32 },

    /// Y-aligned cone (meters).
    ConeY { radius: f32, half_height: f32 },

    /// Rounded cuboid (meters).
    ///
    /// `border_radius` rounds all edges/corners.
    RoundCuboid {
        half_extents: Vector<f32>,
        border_radius: f32,
    },

    /// Y-aligned rounded cylinder (meters).
    RoundCylinderY {
        radius: f32,
        half_height: f32,
        border_radius: f32,
    },

    /// Y-aligned rounded cone (meters).
    RoundConeY {
        radius: f32,
        half_height: f32,
        border_radius: f32,
    },
}

/// In-memory Rapier structures needed for scene queries and KCC against a static world.
///
/// This stores:
/// - `RigidBodySet`/`ColliderSet` containing the static world geometry.
/// - `NarrowPhase` and `BroadPhaseBvh` used to create a borrowed `QueryPipeline`.
///
/// For immutable statics, these can be built once at startup and reused.
pub struct RapierQueryWorld {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub broad_phase: BroadPhaseBvh,
    pub narrow_phase: NarrowPhase,
}

impl RapierQueryWorld {
    /// Build a query world from a list of static collider definitions.
    ///
    /// Determinism
    /// - The input is sorted by `id` before insertion.
    /// - Any NaN/invalid values should be filtered/validated by the caller.
    pub fn build(mut defs: Vec<WorldStaticDef>) -> Self {
        // Ensure deterministic insertion order.
        defs.sort_by_key(|d| d.id);

        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();

        // Insert each static as a fixed rigid-body + attached collider.
        // This is convenient and matches typical Rapier usage.
        for def in defs.into_iter() {
            let iso = Isometry::from_parts(Translation3::from(def.translation), def.rotation);

            let rb = RigidBodyBuilder::fixed().pose(iso).build();
            let rb_handle = bodies.insert(rb);

            let collider = collider_from_def(&def);
            colliders.insert_with_parent(collider, rb_handle, &mut bodies);
        }

        // Initialize broad/narrow phases so queries can run.
        //
        // In Rapier 0.31, we can run collision-detection only (no dynamics) using `CollisionPipeline`.
        // This updates the broad-phase BVH and the narrow-phase contact graph.
        let mut broad_phase = BroadPhaseBvh::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut collision_pipeline = CollisionPipeline::new();

        // Using default hooks/events (none). This will update broad + narrow phases.
        let hooks = ();
        let events = ();

        // NOTE: Rapier 0.31 signature:
        // step(prediction_distance, broad_phase, narrow_phase, bodies, colliders, hooks, events)
        collision_pipeline.step(
            0.0,
            &mut broad_phase,
            &mut narrow_phase,
            &mut bodies,
            &mut colliders,
            &hooks,
            &events,
        );

        Self {
            bodies,
            colliders,
            broad_phase,
            narrow_phase,
        }
    }

    /// Create a borrowed `QueryPipeline` view suitable for scene queries and KCC.
    ///
    /// The returned pipeline borrows `self`, so it should be used within the scope
    /// of the borrow.
    ///
    /// Filters
    /// - Provide a `QueryFilter` to exclude things (e.g., the character collider if you insert it
    ///   into the same scene as the statics).
    pub fn query_pipeline<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
        self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        )
    }
}

/// Build a Rapier collider from a `WorldStaticDef`.
///
/// This uses the pose stored on the rigid-body as the collider parent transform.
/// So the collider is created with identity local transform.
fn collider_from_def(def: &WorldStaticDef) -> Collider {
    match &def.shape {
        ColliderShapeDef::Plane {
            offset_along_normal,
        } => {
            // Derive world-space plane normal from pose rotation: n = R * +Y.
            // Then compute plane dist: n ⋅ x = dist, where x is any point on the plane.
            // With pose translation `t`, dist = n ⋅ t + offset.
            let n = def.rotation * Vector::y();
            let dist = n.dot(&def.translation) + *offset_along_normal;

            // Rapier's half-space expects a `UnitVector<Real>`, not a raw vector.
            // We create it from the (already unit) normal. If the rotation isn't unit-length,
            // this will panic; validate your inputs at ingestion time.
            let unit_n = UnitVector::new_normalize(n);

            // Represent the plane `unit_n ⋅ x = dist` by placing the half-space at translation `unit_n * dist`.
            let halfspace = HalfSpace::new(unit_n);
            ColliderBuilder::new(SharedShape::new(halfspace))
                .translation(unit_n.into_inner() * dist)
                .build()
        }

        ColliderShapeDef::Cuboid { half_extents } => {
            ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z).build()
        }

        ColliderShapeDef::Sphere { radius } => ColliderBuilder::ball(*radius).build(),

        ColliderShapeDef::CapsuleY {
            radius,
            half_height,
        } => ColliderBuilder::capsule_y(*half_height, *radius).build(),

        ColliderShapeDef::CylinderY {
            radius,
            half_height,
        } => ColliderBuilder::cylinder(*half_height, *radius).build(),

        ColliderShapeDef::ConeY {
            radius,
            half_height,
        } => ColliderBuilder::cone(*half_height, *radius).build(),

        ColliderShapeDef::RoundCuboid {
            half_extents,
            border_radius,
        } => ColliderBuilder::round_cuboid(
            half_extents.x,
            half_extents.y,
            half_extents.z,
            *border_radius,
        )
        .build(),

        ColliderShapeDef::RoundCylinderY {
            radius,
            half_height,
            border_radius,
        } => ColliderBuilder::round_cylinder(*half_height, *radius, *border_radius).build(),

        ColliderShapeDef::RoundConeY {
            radius,
            half_height,
            border_radius,
        } => ColliderBuilder::round_cone(*half_height, *radius, *border_radius).build(),
    }
}
