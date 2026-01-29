use crate::foo::{active_character_tbl__view, movement_state_tbl__view, secondary_stats_tbl__view};
use shared::{utils::get_aoi_block, Owner};
use spacetimedb::{SpacetimeType, ViewContext};

#[derive(SpacetimeType, Debug, Default)]
pub struct MovementSpeed {
    pub owner: Owner,
    pub value: f32,
}

/// Finds the movement speed stat for all actors within the AOI.
#[spacetimedb::view(name = movement_speed_view, public)]
pub fn movement_speed_view(ctx: &ViewContext) -> Vec<MovementSpeed> {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return vec![];
    };
    let Some(cell_id) = ctx
        .db
        .movement_state_tbl()
        .owner()
        .find(&active_character.owner)
        .map(|row| row.cell_id)
    else {
        return vec![];
    };

    get_aoi_block(cell_id)
        .into_iter()
        .flat_map(|cell_id| ctx.db.movement_state_tbl().cell_id().filter(cell_id))
        .map(|ms| {
            ctx.db
                .secondary_stats_tbl()
                .owner()
                .find(ms.owner)
                .map(|s| MovementSpeed {
                    owner: ms.owner,
                    value: s.data.movement_speed,
                })
                .unwrap_or_default()
        })
        .collect()
}
