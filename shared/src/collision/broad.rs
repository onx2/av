use nalgebra as na;
use parry3d::{bounding_volume::Aabb, shape as pshape};

use crate::collision::{StaticShape, Transform};

/// Acceleration structure for broad-phase queries over immutable world statics.
///
/// Notes:
/// - We index only finite statics (e.g., Cuboid). Infinite planes are handled separately
///   because they don't have a bounded AABB.
/// - `non_plane_indices` maps each stored AABB back to its index in the original `statics` slice.
/// - `plane_indices` stores indices of planes in the original `statics` slice.
pub struct WorldAccel {
    /// AABBs for non-plane statics (world-space).
    pub aabbs: Vec<Aabb>,
    /// Indices into the original `statics` slice for the AABBs above.
    pub non_plane_indices: Vec<usize>,
    /// Indices into the original `statics` slice for planes.
    pub plane_indices: Vec<usize>,
}

impl WorldAccel {
    /// Return true if this accelerator has no non-plane entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.aabbs.is_empty()
    }

    /// Number of non-plane entries (AABBs) in this accelerator.
    #[inline]
    pub fn len(&self) -> usize {
        self.aabbs.len()
    }
}

/// Build a simple broad-phase accelerator over immutable world statics.
///
/// - Finite shapes (e.g., Cuboid) get a world-space AABB and are indexed.
/// - Infinite shapes (Plane) are kept in `plane_indices` and should be tested separately during queries.
pub fn build_world_accel(statics: &[StaticShape]) -> WorldAccel {
    let mut aabbs = Vec::new();
    let mut non_plane_indices = Vec::new();
    let mut plane_indices = Vec::new();

    for (i, s) in statics.iter().enumerate() {
        match *s {
            StaticShape::Plane { .. } => {
                plane_indices.push(i);
            }
            StaticShape::Cuboid {
                half_extents,
                transform,
            } => {
                if let Some(aabb) = cuboid_aabb_world(half_extents, transform) {
                    aabbs.push(aabb);
                    non_plane_indices.push(i);
                }
            }
        }
    }

    WorldAccel {
        aabbs,
        non_plane_indices,
        plane_indices,
    }
}

/// Compute the AABB for a world-space cuboid.
fn cuboid_aabb_world(half_extents: na::Vector3<f32>, transform: Transform) -> Option<Aabb> {
    let cuboid = pshape::Cuboid::new(half_extents);
    let iso = na::Isometry3::from_parts(
        na::Translation3::new(
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ),
        transform.rotation,
    );
    Some(cuboid.aabb(&iso))
}

/// Compute a swept AABB for a Y-aligned capsule moving from `start_pos` to `start_pos + desired`.
///
/// The resulting AABB is inflated by `skin` to conservatively include near misses.
pub fn swept_capsule_aabb(
    capsule_half_height: f32,
    capsule_radius: f32,
    start_pos: na::Vector3<f32>,
    desired: na::Vector3<f32>,
    skin: f32,
) -> Aabb {
    let capsule = pshape::Capsule::new_y(capsule_half_height, capsule_radius);

    let iso_start = na::Isometry3::from_parts(
        na::Translation3::new(start_pos.x, start_pos.y, start_pos.z),
        na::UnitQuaternion::identity(),
    );
    let end_pos = start_pos + desired;
    let iso_end = na::Isometry3::from_parts(
        na::Translation3::new(end_pos.x, end_pos.y, end_pos.z),
        na::UnitQuaternion::identity(),
    );

    let aabb_start = capsule.aabb(&iso_start);
    let aabb_end = capsule.aabb(&iso_end);

    let mut swept = aabb_union(&aabb_start, &aabb_end);

    if skin > 0.0 {
        swept = aabb_inflate(&swept, skin);
    }

    swept
}

/// Query candidate static indices whose AABB intersects `swept`.
///
/// Returns indices referencing the original `statics` slice (not the local AABB array).
pub fn query_candidates(accel: &WorldAccel, swept: &Aabb) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, aabb) in accel.aabbs.iter().enumerate() {
        if aabb_intersects(aabb, swept) {
            out.push(accel.non_plane_indices[i]);
        }
    }
    out
}

/// Compute the union of two AABBs.
fn aabb_union(a: &Aabb, b: &Aabb) -> Aabb {
    let min = na::Point3::new(
        a.mins.x.min(b.mins.x),
        a.mins.y.min(b.mins.y),
        a.mins.z.min(b.mins.z),
    );
    let max = na::Point3::new(
        a.maxs.x.max(b.maxs.x),
        a.maxs.y.max(b.maxs.y),
        a.maxs.z.max(b.maxs.z),
    );
    Aabb {
        mins: min,
        maxs: max,
    }
}

/// Inflate an AABB by `margin` on all sides.
fn aabb_inflate(a: &Aabb, margin: f32) -> Aabb {
    if margin <= 0.0 {
        return *a;
    }
    let delta = na::Vector3::new(margin, margin, margin);
    Aabb {
        mins: a.mins - delta,
        maxs: a.maxs + delta,
    }
}

/// Test two AABBs for intersection.
fn aabb_intersects(a: &Aabb, b: &Aabb) -> bool {
    !(a.maxs.x < b.mins.x
        || a.mins.x > b.maxs.x
        || a.maxs.y < b.mins.y
        || a.mins.y > b.maxs.y
        || a.maxs.z < b.mins.z
        || a.mins.z > b.maxs.z)
}
