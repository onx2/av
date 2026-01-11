use crate::types::*;
use spacetimedb::*;

/// Live actor entity driven by the server's kinematic controller.
///
/// An `Actor` exists only while the player is "in world". The authoritative
/// values here are updated every tick by the server and may be mirrored
/// back to the `Player` row when leaving or disconnecting.
#[table(name = actor)]
pub struct Actor {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    #[unique]
    pub primary_stats_id: u32,
    #[unique]
    pub secondary_stats_id: u32,
    #[unique]
    pub vital_stats_id: u32,
    #[unique]
    pub transform_data_id: u64,

    /// An optional player identity when this actor is controlled, NOT a server actor.
    pub identity: Option<Identity>,

    #[index(btree)]
    pub should_move: bool,

    pub move_intent: MoveIntent,

    pub grounded: bool,

    #[index(btree)]
    pub cell_id: u32,

    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}
