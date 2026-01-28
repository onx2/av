use super::Capsule;
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType};

/// Ephemeral/computed & cached state for the owner's movement. This doesn't need to be persisted
/// and can be removed when the owner is removed from the world.
#[table(name=movement_state_tbl)]
pub struct MovementState {
    #[primary_key]
    pub owner: Owner,

    #[index(btree)]
    pub cell_id: u32,

    /// Is the owner in contact with a surface, I.E. not "falling"
    pub grounded: bool,

    /// Tracked for gravity acceleration
    pub vertical_velocity: f32,

    pub collider: ColliderData,
}

impl MovementState {
    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.movement_state_tbl().owner().find(owner)
    }
}
#[derive(SpacetimeType, Debug, Copy, Clone)]
pub struct ColliderData {
    /// Capsule shape of the collider for the owner, restricting this to capsules to simplify and
    /// lower costs in spacetimeDB. Capsule = 8 bytes, but quantized to 4bytes using u16.
    pub capsule: Capsule,

    /// Is the collider a sensor?
    pub is_sensor: bool,
}
