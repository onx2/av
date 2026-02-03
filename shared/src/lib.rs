pub mod collision;
pub mod constants;
pub mod utils;

pub use collision::{ColliderShapeDef, WorldStaticDef, collider_from_def};
pub use constants::{
    CELL_SIZE, DIRECTIONAL_MOVEMENT_INTERVAL, MAX_INTENT_DISTANCE_SQ, MAX_INTENT_PATH_LEN,
    SMALLEST_MOVE_DISTANCE_SQ, SMALLEST_REQUEST_DISTANCE_SQ, WORLD_OFFSET,
};
pub use utils::*;

/// 4byte unique identifier for an actor.
/// ~ 4billion records allowed + auto_inc wraps around but doesn't verify insert so this
/// could fail at some point, but when that happens I'll be able to afford bumping this to u64.
pub type ActorId = u32;
