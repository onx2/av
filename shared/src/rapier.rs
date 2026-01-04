use rapier3d::{na::UnitQuaternion, prelude::*};

/// Canonical, schema-agnostic definition of an immutable world collider.
#[derive(Clone, Debug)]
pub struct WorldStaticDef {
    pub id: u32,
    /// World-space translation.
    pub translation: Vector<f32>,
    /// World-space rotation (unit quaternion).
    pub rotation: UnitQuaternion<f32>,
    /// Collider shape parameters.
    pub shape: ColliderShapeDef,
}

/// Supported static collider shapes.
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

/// Build a Rapier collider from a `WorldStaticDef`.
///
/// This uses the pose stored on the rigid-body as the collider parent transform.
/// So the collider is created with identity local transform.
pub fn collider_from_def(def: &WorldStaticDef) -> Collider {
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
