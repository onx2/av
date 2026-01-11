use crate::{
    schema::{actor__view, secondary_stats__view},
    types::AoiSecondaryStats,
    views::actor_from_ctx,
};
use shared::utils::get_aoi_block;

#[spacetimedb::view(name = aoi_secondary_stats, public)]
fn aoi_secondary_stats(ctx: &spacetimedb::ViewContext) -> Vec<AoiSecondaryStats> {
    let Some(local_actor) = actor_from_ctx(ctx) else {
        return Vec::new();
    };

    let aoi_block: [u32; 9] = get_aoi_block(local_actor.cell_id);
    aoi_block
        .into_iter()
        .flat_map(|cell_id| {
            ctx.db.actor().cell_id().filter(cell_id).map(|actor| {
                let secondary_stats = ctx
                    .db
                    .secondary_stats()
                    .id()
                    .find(actor.secondary_stats_id)
                    .unwrap_or_default();
                AoiSecondaryStats {
                    id: secondary_stats.id,
                    actor_id: actor.id,
                    transform_data_id: actor.transform_data_id,
                    movement_speed: secondary_stats.movement_speed,
                    max_health: secondary_stats.max_health,
                    max_mana: secondary_stats.max_mana,
                    max_stamina: secondary_stats.max_stamina,
                }
            })
        })
        .collect()
}
