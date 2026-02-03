pub mod cell;
pub mod collision;
pub mod constants;
pub mod utils;

pub use cell::{
    decode_cell_coords, decode_cell_min_corner, encode_cell_id, get_aoi_block, max_cell_coord,
    world_span_m,
};
pub use collision::{ColliderShapeDef, WorldStaticDef, collider_from_def};
pub use constants::{
    CELL_SIZE, DIRECTIONAL_MOVEMENT_INTERVAL, GRID_SIDE, INV_CELL_SIZE, MAX_INTENT_DISTANCE_SQ,
    MAX_INTENT_PATH_LEN, SMALLEST_MOVE_DISTANCE_SQ, SMALLEST_REQUEST_DISTANCE_SQ, WORLD_OFFSET,
};
pub use utils::*;

/// 4byte unique identifier for an actor.
/// ~ 4billion records allowed + auto_inc wraps around but doesn't verify insert so this
/// could fail at some point, but when that happens I'll be able to afford bumping this to u64.
pub type ActorId = u32;

/// Compact cell identifier for AOI + spatial views.
pub type CellId = u16;
