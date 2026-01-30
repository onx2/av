use crate::{active_character_tbl__view, movement_state_tbl__view};
use shared::get_aoi_block;
use spacetimedb::ViewContext;

/// Finds this character's AOI block for views
///
/// **Performance & Cost**: O(1), two index seeks
pub fn get_view_aoi_block(ctx: &ViewContext) -> Option<impl Iterator<Item = u32>> {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return None;
    };
    let Some(cell_id) = ctx
        .db
        .movement_state_tbl()
        .owner()
        .find(&active_character.owner)
        .map(|row| row.cell_id)
    else {
        return None;
    };

    Some(get_aoi_block(cell_id).into_iter())
}
