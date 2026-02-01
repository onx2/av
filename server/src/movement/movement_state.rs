use crate::{Capsule, MoveIntentData};
use shared::Owner;
use spacetimedb::{table, ReducerContext, ViewContext};

/// Ephemeral/computed & cached state for the owner's movement. This doesn't need to be persisted
/// and can be removed when the owner is removed from the world.
#[table(name=movement_state_tbl)]
pub struct MovementStateRow {
    #[primary_key]
    pub owner: Owner,

    #[index(btree)]
    pub cell_id: u32,

    /// Index-able column for the `move_intent` because SpacetimeType cannot be indexed.
    /// This is true when grounded=false || Some(move_intent)
    #[index(btree)]
    pub should_move: bool,

    pub move_intent: Option<MoveIntentData>,

    /// Is the owner in contact with a surface, I.E. not "falling"
    pub grounded: bool,

    /// Tracked for gravity acceleration
    pub vertical_velocity: f32,

    /// Capsule shape of the collider for the owner, restricting this to capsules to simplify and
    /// lower costs in spacetimeDB. Capsule = 8 bytes, but quantized to 4bytes using u16.
    pub capsule: Capsule,
}

impl MovementStateRow {
    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.movement_state_tbl().owner().find(owner)
    }

    /// Find all movement states for a given cell ID.
    ///
    /// **Performance & Cost**: O(log N), bsatn seek (index?? TBD)
    pub fn by_cell_id(ctx: &ViewContext, cell_id: u32) -> impl Iterator<Item = Self> {
        ctx.db.movement_state_tbl().cell_id().filter(cell_id)
    }
}
