use super::{ComputedStat, Stat};
use crate::foo::{active_character_tbl__view, level_tbl__view, primary_stats_tbl__view};
use shared::Owner;
use spacetimedb::{DbContext, LocalReadOnly, ViewContext};

pub struct MovementSpeed;
impl ComputedStat for MovementSpeed {
    type Output = Stat<f32>;
    fn compute(db: &LocalReadOnly, owner: Owner) -> Option<Self::Output> {
        let Some(primary_stats) = db.primary_stats_tbl().owner().find(owner) else {
            return None;
        };
        let Some(level) = db.level_tbl().owner().find(owner) else {
            return None;
        };

        let dex_multiplier = primary_stats.data.dexterity as f32 / 10.0;
        let level_multiplier = level.data.level as f32 / 10.0;
        Some(Stat::new(3.0 * dex_multiplier * level_multiplier))
    }
}
#[spacetimedb::view(name = movement_speed_view, public)]
pub fn movement_speed_view(ctx: &ViewContext) -> Option<<MovementSpeed as ComputedStat>::Output> {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return None;
    };

    MovementSpeed::compute(ctx.db(), active_character.owner)
}
