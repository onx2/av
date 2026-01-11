use shared::utils::get_aoi_block;
use spacetimedb::{view, ViewContext};

use crate::{
    schema::{
        actor__view, player__view, secondary_stats__view, transform_data__view, Actor,
        TransformData,
    },
    types::{AoiActor, AoiSecondaryStats},
};

fn actor_from_ctx(ctx: &ViewContext) -> Option<Actor> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return None;
    };
    let Some(actor_id) = player.actor_id else {
        return None;
    };
    ctx.db.actor().id().find(actor_id)
}

#[view(name = aoi_actor, public)]
fn aoi_actor_view(ctx: &ViewContext) -> Vec<AoiActor> {
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

#[view(name = aoi_secondary_stats, public)]
fn aoi_secondary_stats_view(ctx: &ViewContext) -> Vec<AoiSecondaryStats> {
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

#[view(name = aoi_transform_data, public)]
fn aoi_transform_data_view(ctx: &ViewContext) -> Vec<TransformData> {
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
