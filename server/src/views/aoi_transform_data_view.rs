use crate::{
    schema::{actor__view, transform_data__view},
    views::actor_from_ctx,
    TransformData,
};
use shared::utils::get_aoi_block;

#[spacetimedb::view(name = aoi_transform_data, public)]
fn aoi_transform_data(ctx: &spacetimedb::ViewContext) -> Vec<TransformData> {
    let Some(actor) = actor_from_ctx(ctx) else {
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
