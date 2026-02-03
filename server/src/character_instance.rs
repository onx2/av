use crate::{get_view_aoi_block, ActorId, MovementStateRow};
use spacetimedb::{table, Identity, ViewContext};

#[table(name=character_instance_tbl)]
pub struct CharacterInstanceRow {
    #[primary_key]
    pub identity: Identity,

    #[unique]
    pub actor_id: ActorId,

    #[unique]
    pub character_id: u32,
}

impl CharacterInstanceRow {
    pub fn find_by_identity(ctx: &ViewContext) -> Option<Self> {
        ctx.db.character_instance_tbl().identity().find(ctx.sender)
    }
    pub fn find_by_actor_id(ctx: &ViewContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.character_instance_tbl().actor_id().find(actor_id)
    }

    pub fn new(identity: Identity, actor_id: ActorId, character_id: u32) -> Self {
        Self {
            identity,
            actor_id,
            character_id,
        }
    }
}

/// Finds the active character for all things within the AOI.
/// Primary key of `Identity`
#[spacetimedb::view(name = character_instance_view, public)]
pub fn character_instance_view(ctx: &ViewContext) -> Vec<CharacterInstanceRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };
    log::info!("character_instance_view called");

    cell_block
        .flat_map(|cell_id| MovementStateRow::by_cell_id(ctx, cell_id))
        .filter_map(|ms| CharacterInstanceRow::find_by_actor_id(ctx, ms.actor_id))
        .collect()
}
