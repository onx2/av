use shared::utils::get_aoi_block;
use spacetimedb::{view, ViewContext};

use crate::{
    schema::{actor__view, player__view, transform_data__view, TransformData},
    types::AoiActor,
};

#[view(name = aoi_actor, public)]
fn aoi_actor_view(ctx: &ViewContext) -> Vec<AoiActor> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return Vec::new();
    };
    let Some(actor_id) = player.actor_id else {
        return Vec::new();
    };
    let Some(actor) = ctx.db.actor().id().find(actor_id) else {
        return Vec::new();
    };

    let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);

    aoi_block
        .into_iter()
        .flat_map(|cell_id| ctx.db.actor().cell_id().filter(cell_id))
        .map(|a| a.into())
        .collect()
}

#[view(name = aoi_transform_data, public)]
fn aoi_transform_data_view(ctx: &ViewContext) -> Vec<TransformData> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return Vec::new();
    };
    let Some(actor_id) = player.actor_id else {
        return Vec::new();
    };
    let Some(actor) = ctx.db.actor().id().find(actor_id) else {
        return Vec::new();
    };

    let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);
    aoi_block
        .into_iter()
        .flat_map(|cell_id| {
            ctx.db.actor().cell_id().filter(cell_id).map(|actor| {
                ctx.db
                    .transform_data()
                    .id()
                    .find(actor.transform_data_id)
                    .unwrap_or_default()
            })
        })
        .collect()
}
