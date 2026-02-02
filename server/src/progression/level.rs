use crate::{
    HealthData, HealthRow, ManaData, ManaRow, PrimaryStatsRow, SecondaryStatsData,
    SecondaryStatsRow, MAX_LEVEL, TIER_INTERVAL,
};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table, ViewContext};

/// The amount of progression this person has accumulated
#[table(name = level_tbl)]
pub struct LevelRow {
    #[primary_key]
    pub owner: Owner,

    pub data: LevelData,
}
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelData {
    pub level: u8,
}

impl Default for LevelData {
    fn default() -> Self {
        Self { level: 1 }
    }
}
impl LevelData {
    /// What is the total amount of points alloted for every level up to and including this level.
    /// 1 point per level, except each tier level (10, 20, 30, ...) gives 3 instead of 1.
    /// That means +2 bonus points per tier reached.
    pub fn points_for_level(level: u8) -> u8 {
        level + (level / TIER_INTERVAL) * 2
    }
}

impl LevelRow {
    pub fn find(ctx: &ViewContext, owner: Owner) -> Option<Self> {
        ctx.db.level_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: LevelData) {
        ctx.db.level_tbl().insert(Self { owner, data });
    }

    pub fn update(&self, ctx: &ReducerContext, new_level: u8) {
        if new_level == self.data.level {
            log::warn!("Unable to change level to the same value");
            return;
        }
        if new_level > MAX_LEVEL {
            log::warn!("Unable to change level to a value greater than the max");
            return;
        }

        let res = ctx.db.level_tbl().owner().update(Self {
            owner: self.owner,
            data: LevelData { level: new_level },
        });
        let Some(primary_stats_data) =
            PrimaryStatsRow::find(&ctx.as_read_only(), self.owner).map(|row| row.data)
        else {
            log::error!(
                "Failed to find fortitude for player on level change {}",
                self.owner
            );
            return;
        };

        let view_ctx = ctx.as_read_only();
        // Updates to the level should trigger a recompute of the max health
        if let Some(health) = HealthRow::find(&view_ctx, self.owner) {
            health.set_max(
                ctx,
                HealthData::compute_max(res.data.level, primary_stats_data.fortitude),
            );
        }
        if let Some(mana) = ManaRow::find(&view_ctx, self.owner) {
            mana.set_max(
                ctx,
                ManaData::compute_max(res.data.level, primary_stats_data.intellect),
            );
        }

        // Update seconday stats when we change level
        if let Some(mut secondary_stats) = SecondaryStatsRow::find(&view_ctx, self.owner) {
            secondary_stats.data.movement_speed =
                SecondaryStatsData::compute_movement_speed(res.data.level, 0., 0., 0.);
            secondary_stats.data.critical_hit_chance =
                SecondaryStatsData::compute_critical_hit_chance(
                    res.data.level,
                    primary_stats_data.ferocity,
                    0.,
                );
            SecondaryStatsRow::update_from_self(secondary_stats, ctx);
        }
    }
}
