use crate::{schema::actor__view, types::AoiActor, views::actor_from_ctx};
use shared::utils::get_aoi_block;

#[spacetimedb::view(name = aoi_actor, public)]
fn aoi_actor(ctx: &spacetimedb::ViewContext) -> Vec<AoiActor> {
    let Some(actor) = actor_from_ctx(ctx) else {
        return Vec::new();
    };

    let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);
    aoi_block
        .into_iter()
        .flat_map(|cell_id| ctx.db.actor().cell_id().filter(cell_id))
        .map(|a| a.into())
        .collect()
}
