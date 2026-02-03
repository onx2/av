use crate::{character_instance_tbl__view, movement_state_tbl__view};
use shared::{get_aoi_block, CellId};
use spacetimedb::ViewContext;

/// Finds this character's AOI block for views
///
/// **Performance & Cost**: O(1), two index seeks
pub fn get_view_aoi_block(ctx: &ViewContext) -> Option<impl Iterator<Item = CellId>> {
    let Some(ci) = ctx.db.character_instance_tbl().identity().find(ctx.sender) else {
        return None;
    };
    let Some(cell_id) = ctx
        .db
        .movement_state_tbl()
        .actor_id()
        .find(&ci.actor_id)
        .map(|row| row.cell_id)
    else {
        return None;
    };

    Some(get_aoi_block(cell_id).into_iter())
}
