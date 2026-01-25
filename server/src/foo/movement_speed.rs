use super::{
    active_character_tbl, active_character_tbl__view, level_tbl__view, primary_stats_tbl__view,
};
use shared::owner::Owner;
use spacetimedb::{DbContext, LocalReadOnly, ReducerContext, SpacetimeType, ViewContext};

#[derive(SpacetimeType)]
pub struct MovementSpeed {
    pub speed: f32,
}

pub fn compute_movement_speed(db: &LocalReadOnly, owner: Owner) -> Option<f32> {
    let Some(primary_stats) = db.primary_stats_tbl().owner().find(owner) else {
        return None;
    };
    let Some(level) = db.level_tbl().owner().find(owner) else {
        return None;
    };

    let dex_multiplier = primary_stats.data.dexterity as f32 / 10.0;
    let level_multiplier = level.data.level as f32 / 10.0;
    Some(3.0 * dex_multiplier * level_multiplier)
}

#[spacetimedb::view(name = movement_speed_view, public)]
pub fn movement_speed_view(ctx: &ViewContext) -> Option<MovementSpeed> {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return None;
    };

    compute_movement_speed(ctx.db(), active_character.owner).map(|speed| MovementSpeed { speed })
}

#[spacetimedb::reducer]
pub fn movement_speed_red(ctx: &ReducerContext) {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return;
    };
    // ex: how to get the computed movement speed from a reducer context
    let movement_speed = compute_movement_speed(&ctx.as_read_only().db, active_character.owner);
}
