use log::info;
use nalgebra::point;
use rapier3d::prelude::*;
use spacetimedb::{
    log_stopwatch::LogStopwatch as SpacetimeLogStopwatch, ReducerContext, ScheduleAt,
};

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

/// Returns whether there is ground support within `max_dist` under the capsule.
///
/// `translation_*` must be in world meters (decoded). This keeps the function usable regardless of
/// whether the caller stores positions as floats or quantized integers.
pub fn has_support_within(
    query_pipeline: &QueryPipeline<'_>,
    translation_x_m: f32,
    translation_y_m: f32,
    translation_z_m: f32,
    capsule_half_height: f32,
    capsule_radius: f32,
    max_dist: f32,
    min_ground_normal_y: f32,
) -> bool {
    // Probe from the capsule "feet" (slightly above to avoid starting inside geometry).
    let feet_y: f32 = translation_y_m - (capsule_half_height + capsule_radius);
    let origin_y = feet_y + 0.02;

    let ray = Ray::new(
        point![translation_x_m, origin_y, translation_z_m],
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

/// LogStopwatch-style sampled span logging (WASM-safe).
///
/// This mirrors the approach from the reference repo:
/// - logs a begin/end wrapper for the event
/// - logs total event time via SpacetimeDB's `log_stopwatch`
/// - supports sequential spans (`span()` ends the previous span)
///
/// This is intended for ad-hoc profiling without adding any monotonic clock dependencies.
pub struct LogStopwatch {
    event_sw: Option<SpacetimeLogStopwatch>,
    span_sw: Option<SpacetimeLogStopwatch>,
    name: String,
    should_sample: bool,
}

impl LogStopwatch {
    /// Creates a new `LogStopwatch` that conditionally logs timing information.
    ///
    /// Sampling:
    /// - If `force_debug` is true, always logs.
    /// - Otherwise logs with probability `sample_rate` in [0, 1].
    ///
    /// Note: sampling uses `ctx.random::<f32>()` so the module remains deterministic.
    pub fn new(
        ctx: &ReducerContext,
        name: impl Into<String>,
        force_debug: bool,
        sample_rate: f32,
    ) -> Self {
        let name = name.into();
        let should_sample =
            force_debug || (sample_rate > 0.0 && ctx.random::<f32>() <= sample_rate);

        if should_sample {
            info!("--------- {name} begin ---------");
        }

        Self {
            event_sw: if should_sample {
                Some(SpacetimeLogStopwatch::new("event_time"))
            } else {
                None
            },
            span_sw: None,
            name,
            should_sample,
        }
    }

    /// Starts a new span within the event, ending any previous span.
    pub fn span(&mut self, section_name: &str) {
        if !self.should_sample {
            return;
        }

        if let Some(sw) = self.span_sw.take() {
            sw.end();
        }

        self.span_sw = Some(SpacetimeLogStopwatch::new(section_name));
    }

    /// Ends the current span, if any.
    pub fn end_span(&mut self) {
        if let Some(sw) = self.span_sw.take() {
            sw.end();
        }
    }

    /// Whether this event is currently being sampled/logged.
    pub fn should_sample(&self) -> bool {
        self.should_sample
    }
}

impl Drop for LogStopwatch {
    fn drop(&mut self) {
        if !self.should_sample {
            return;
        }

        // Close any open span first.
        if let Some(sw) = self.span_sw.take() {
            sw.end();
        }

        // Close event timer.
        if let Some(sw) = self.event_sw.take() {
            sw.end();
        }

        info!("---------- {} end ----------", self.name);
    }
}
