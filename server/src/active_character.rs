use shared::Owner;
use spacetimedb::{table, Identity, ViewContext};

use crate::{get_view_aoi_block, MovementStateRow};

/// Marker table for the active character for a given player
#[table(name=active_character_tbl)]
pub struct ActiveCharacterRow {
    #[primary_key]
    pub identity: Identity,

    #[unique]
    pub owner: Owner,
}
impl ActiveCharacterRow {
    pub fn new(identity: Identity, owner: Owner) -> Self {
        Self { identity, owner }
    }
}

/// Finds the active character for all things within the AOI.
/// Primary key of `Identity`
#[spacetimedb::view(name = active_character_view, public)]
pub fn active_character_view(ctx: &ViewContext) -> Vec<ActiveCharacterRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementStateRow::by_cell_id(ctx, cell_id))
        .filter_map(|ms| ctx.db.active_character_tbl().owner().find(&ms.owner))
        .collect()
}
