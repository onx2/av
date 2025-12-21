use crate::types::DbVec3;
use nalgebra::point;
use rapier3d::prelude::*;
use spacetimedb::ScheduleAt;

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
    let feet_y = translation.y - (capsule_half_height + capsule_radius);
    let origin_y = feet_y + 0.02;

    let ray = Ray::new(
        point![translation.x, origin_y, translation.z],
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
