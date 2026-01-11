mod aoi_actor_view;
mod aoi_secondary_stats_view;
mod aoi_transform_data_view;

// Bring view-table extension traits into scope so `ctx.db.actor()` / `ctx.db.player()` etc compile.
use crate::schema::{actor__view, player__view, Actor};
use spacetimedb::ViewContext;

pub(super) fn actor_from_ctx(ctx: &ViewContext) -> Option<Actor> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return None;
    };
    let Some(actor_id) = player.actor_id else {
        return None;
    };
    ctx.db.actor().id().find(actor_id)
}
