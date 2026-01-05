mod reducers {
    mod connection;
    pub mod enter_world;
    pub mod leave_world;
    pub(crate) mod movement_tick;
    pub mod request_move;
}
pub mod schema;
pub mod types;
mod utils;
pub mod views;
mod world;

use crate::schema::*;
use reducers::movement_tick::init_movement_tick;
use spacetimedb::*;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    ctx.db.kcc_settings().id().delete(1);
    ctx.db.kcc_settings().insert(KccSettings {
        id: 1,
        offset: 0.05,
        max_slope_climb_deg: 52.0,
        min_slope_slide_deg: 45.0,
        autostep_max_height: 0.325,
        autostep_min_width: 0.2,
        slide: true,
        normal_nudge_factor: 0.05,
        fall_speed_mps: 12.,
        grounded_down_bias_mps: 1.75,
        point_acceptance_radius_sq: 0.0225,
    });
    world::recreate_static_world(ctx);
    init_movement_tick(ctx);
}
