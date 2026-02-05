use crate::{get_view_aoi_block, MoveIntentData};
use shared::{ActorId, CellId};
use spacetimedb::{table, ReducerContext, ViewContext};

/// Ephemeral/computed & cached state for the owner's movement. This doesn't need to be persisted
/// and can be removed when the owner is removed from the world.
#[table(name=movement_state_tbl)]
pub struct MovementStateRow {
    #[primary_key]
    pub actor_id: ActorId,

    #[index(btree)]
    pub cell_id: CellId,

    pub move_intent: Option<MoveIntentData>,

    /// Index-able column for the `move_intent` because SpacetimeType cannot be indexed.
    /// This is true when grounded=false || Some(move_intent)
    #[index(btree)]
    pub should_move: bool,

    /// Quantized vertical velocity (meters/second).
    ///
    /// - `0` means grounded / no vertical motion.
    /// - Negative values mean falling downward.
    ///
    /// This is intentionally quantized to save bytes. The server derives per-tick vertical
    /// displacement from this plus `dt`.
    pub vertical_velocity: i8,
}

impl MovementStateRow {
    pub fn find(ctx: &ReducerContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.movement_state_tbl().actor_id().find(actor_id)
    }

    /// Updates from given self, caller should have updated the state with the latest values.
    pub fn update_from_self(self, ctx: &ReducerContext) {
        ctx.db.movement_state_tbl().actor_id().update(self);
    }

    /// Find all movement states for a given cell ID.
    ///
    /// **Performance & Cost**: O(log N), bsatn seek (index?? TBD)
    pub fn by_cell_id(ctx: &ViewContext, cell_id: CellId) -> impl Iterator<Item = Self> {
        ctx.db.movement_state_tbl().cell_id().filter(cell_id)
    }
}

/// Finds the secondary stats for all actors within the AOI.
/// Primary key of `ActorId`
#[spacetimedb::view(name = movement_state_view, public)]
pub fn movement_state_view(ctx: &ViewContext) -> Vec<MovementStateRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementStateRow::by_cell_id(ctx, cell_id))
        .collect()
}
