use crate::{
    HealthData, HealthRow, ManaData, ManaRow, PrimaryStatsRow, SecondaryStatsData,
    SecondaryStatsRow, MAX_LEVEL, TIER_INTERVAL,
};
use shared::ActorId;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table, ViewContext};

/// The amount of progression this person has accumulated
#[table(name = level_tbl)]
pub struct LevelRow {
    #[primary_key]
    pub actor_id: ActorId,

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
    pub fn find(ctx: &ViewContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.level_tbl().actor_id().find(actor_id)
    }
    pub fn insert(ctx: &ReducerContext, actor_id: ActorId, data: LevelData) {
        ctx.db.level_tbl().insert(Self { actor_id, data });
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

        let res = ctx.db.level_tbl().actor_id().update(Self {
            actor_id: self.actor_id,
            data: LevelData { level: new_level },
        });
        let Some(primary_stats_data) =
            PrimaryStatsRow::find(&ctx.as_read_only(), self.actor_id).map(|row| row.data)
        else {
            log::error!(
                "Failed to find fortitude for player on level change {}",
                self.actor_id
            );
            return;
        };

        let view_ctx = ctx.as_read_only();
        // Updates to the level should trigger a recompute of the max health
        if let Some(health) = HealthRow::find(&view_ctx, self.actor_id) {
            health.set_max(
                ctx,
                HealthData::compute_max(res.data.level, primary_stats_data.fortitude),
            );
        }
        if let Some(mana) = ManaRow::find(&view_ctx, self.actor_id) {
            mana.set_max(
                ctx,
                ManaData::compute_max(res.data.level, primary_stats_data.intellect),
            );
        }

        // Update secondary stats when we change level
        if let Some(mut secondary_stats) = SecondaryStatsRow::find(&view_ctx, self.actor_id) {
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
