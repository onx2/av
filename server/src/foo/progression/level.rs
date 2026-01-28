use crate::foo::{health_tbl, primary_stats_tbl, HealthData};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table};

/// The amount of progression this person has accumulated
#[table(name = level_tbl)]
pub struct Level {
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

impl Level {
    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.level_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: LevelData) {
        ctx.db.level_tbl().insert(Self { owner, data });
    }
    pub fn update(&self, ctx: &ReducerContext, data: LevelData) {
        let res = ctx.db.level_tbl().owner().update(Self {
            owner: self.owner,
            data,
        });
        let level = res.data.level;
        let Some(fortitude) = ctx
            .db
            .primary_stats_tbl()
            .owner()
            .find(self.owner)
            .map(|row| row.data.fortitude)
        else {
            log::error!(
                "Failed to find fortitude for player on level change {}",
                self.owner
            );
            return;
        };

        // Updates to the level should trigger a recompute of the max health
        if let Some(health) = ctx.db.health_tbl().owner().find(self.owner) {
            health.set_max(ctx, HealthData::compute_max(level, fortitude));
        }
    }
}
