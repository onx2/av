use crate::{schema::world_static, types::DbVec3, world::row_to_def};
use nalgebra::{point, Translation3};
use rapier3d::prelude::*;
use shared::{rapier::collider_from_def, utils::get_aoi_block};
use spacetimedb::{ReducerContext, ScheduleAt};

pub fn get_variable_delta_time(
    now: spacetimedb::Timestamp,
    last: spacetimedb::Timestamp,
) -> Option<f32> {
    now.time_duration_since(last)
        .map(|dur| dur.to_micros() as f32 / 1_000_000.0)
}

pub fn get_fixed_delta_time(scheduled_at: ScheduleAt) -> f32 {
    match scheduled_at {
        ScheduleAt::Interval(dt) => dt.to_micros() as f32 / 1_000_000.0,
        _ => panic!("Expected ScheduleAt to be Interval"),
    }
}

pub fn has_support_within(
    query_pipeline: &QueryPipeline<'_>,
    translation: &DbVec3,
    capsule_half_height: f32,
    capsule_radius: f32,
    max_dist: f32,
    min_ground_normal_y: f32,
) -> bool {
    // Probe from the capsule "feet" (slightly above to avoid starting inside geometry).
    let feet_y: f32 = translation.y as f32 - (capsule_half_height + capsule_radius);
    let origin_y = feet_y + 0.02;

    let ray = Ray::new(
        point![translation.x.into(), origin_y, translation.z.into()],
        vector![0.0, -1.0, 0.0],
    );

    if let Some((_handle, hit)) =
        query_pipeline.cast_ray_and_get_normal(&ray, max_dist.max(0.0), true)
    {
        hit.normal.y >= min_ground_normal_y
    } else {
        false
    }
}

/// Owns the Rapier structures needed to create a `QueryPipeline<'_>` in Rapier 0.31.
///
/// In 0.31, `QueryPipeline` borrows the broad-phase BVH and the sets, so you can't
/// return it directly from a builder function without also returning the owned data.
pub struct StaticQueryWorld {
    bodies: RigidBodySet,
    colliders: ColliderSet,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
}

impl StaticQueryWorld {
    pub fn as_query_pipeline(&self) -> QueryPipeline<'_> {
        self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            QueryFilter::default(),
        )
    }
}

pub fn build_static_query_world(
    ctx: &ReducerContext,
    dt: f32,
    actor_cell_id: u32,
) -> StaticQueryWorld {
    let bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut modified_colliders = Vec::new();

    let by_global_and_cell = ctx.db.world_static().is_global_and_cell_id();

    for row in by_global_and_cell.filter((true, 0u32..)) {
        let def = row_to_def(row);
        let mut collider = collider_from_def(&def);
        let iso = Isometry::from_parts(Translation3::from(def.translation), def.rotation);
        collider.set_position(iso);
        let co_handle = colliders.insert(collider);
        modified_colliders.push(co_handle);
    }

    get_aoi_block(actor_cell_id).iter().for_each(|cell_id| {
        for row in by_global_and_cell.filter((false, *cell_id)) {
            let def = row_to_def(row);
            let mut collider = collider_from_def(&def);
            let iso = Isometry::from_parts(Translation3::from(def.translation), def.rotation);
            collider.set_position(iso);
            let co_handle = colliders.insert(collider);
            modified_colliders.push(co_handle);
        }
    });

    let mut broad_phase = BroadPhaseBvh::new();
    let mut events = Vec::new();
    broad_phase.update(
        &IntegrationParameters {
            dt,
            ..IntegrationParameters::default()
        },
        &colliders,
        &bodies,
        &modified_colliders,
        &[],
        &mut events,
    );

    log::info!("modified_colliders #: {:?}", modified_colliders.len());

    StaticQueryWorld {
        bodies,
        colliders,
        broad_phase,
        narrow_phase: NarrowPhase::default(),
    }
}
