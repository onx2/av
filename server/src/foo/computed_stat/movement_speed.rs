use super::ComputedStat;
use crate::foo::{get_computed_stat_view, level_tbl__view, primary_stats_tbl__view};
use shared::Owner;
use spacetimedb::{LocalReadOnly, SpacetimeType, ViewContext};

#[derive(SpacetimeType, Debug, Default)]
pub struct MovementSpeed {
    pub owner: Owner,
    pub value: f32,
}

impl ComputedStat for MovementSpeed {
    type Output = MovementSpeed;

    fn compute(db: &LocalReadOnly, owner: Owner) -> Option<Self::Output> {
        let Some(primary_stats) = db.primary_stats_tbl().owner().find(owner) else {
            return None;
        };
        let Some(level) = db.level_tbl().owner().find(owner) else {
            return None;
        };

        let base_speed = 4.0;
        // +2% per point of DEX
        let dex_bonus = primary_stats.data.dexterity as f32 * 0.02;
        // +1% per Level, at lvl 50 = 50% bonus so 6m/s total
        let level_bonus = level.data.level as f32 * 0.01;

        // TODO: buffs / modifiers from spells or equipment
        // Velocity spell might give +10% speed, boots might give +5% speed
        // I might just have a "modifier" table that has rows attached to the owner
        // such that I can find all modifiers and then filter by the type of modifier
        // to find how we add here...

        // Compute the speed from multipliers but cap at 10m/s
        let speed = base_speed * (1.0 + dex_bonus + level_bonus).min(10.0);

        Some(MovementSpeed {
            owner,
            value: speed,
        })
    }
}

/// Finds the movement speed stat for all actors within the AOI.
#[spacetimedb::view(name = movement_speed_view, public)]
pub fn movement_speed_view(ctx: &ViewContext) -> Vec<MovementSpeed> {
    get_computed_stat_view::<MovementSpeed>(ctx)
}
