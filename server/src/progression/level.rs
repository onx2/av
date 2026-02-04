use crate::{
    get_view_aoi_block, HealthData, HealthRow, ManaData, ManaRow, MovementStateRow,
    PrimaryStatsRow, SecondaryStatsRow, MAX_LEVEL, TIER_INTERVAL,
};
use shared::ActorId;
use spacetimedb::{table, ReducerContext, Table, ViewContext};

/// The amount of progression this person has accumulated
#[table(name = level_tbl)]
pub struct LevelRow {
    #[primary_key]
    pub actor_id: ActorId,

    pub level: u8,
}

impl LevelRow {
    /// What is the total amount of points alloted for every level up to and including this level.
    /// 1 point per level, except each tier level (10, 20, 30, ...) gives 3 instead of 1.
    /// That means +2 bonus points per tier reached.
    pub fn points_for_level(level: u8) -> u8 {
        level + (level / TIER_INTERVAL) * 2
    }

    pub fn find(ctx: &ViewContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.level_tbl().actor_id().find(actor_id)
    }
    pub fn insert(ctx: &ReducerContext, actor_id: ActorId, level: u8) {
        ctx.db.level_tbl().insert(Self { actor_id, level });
    }

    pub fn update(&self, ctx: &ReducerContext, new_level: u8) {
        if new_level == self.level {
            log::warn!("Unable to change level to the same value");
            return;
        }
        if new_level > MAX_LEVEL {
            log::warn!("Unable to change level to a value greater than the max");
            return;
        }

        let res = ctx.db.level_tbl().actor_id().update(Self {
            actor_id: self.actor_id,
            level: new_level,
        });
        let Some(primary_stats) = PrimaryStatsRow::find(&ctx.as_read_only(), self.actor_id) else {
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
                HealthData::compute_max(res.level, primary_stats.fortitude),
            );
        }
        if let Some(mana) = ManaRow::find(&view_ctx, self.actor_id) {
            mana.set_max(
                ctx,
                ManaData::compute_max(res.level, primary_stats.intellect),
            );
        }

        // Update secondary stats when we change level
        if let Some(mut secondary_stats) = SecondaryStatsRow::find(&view_ctx, self.actor_id) {
            secondary_stats.movement_speed =
                SecondaryStatsRow::compute_movement_speed(res.level, 0., 0., 0.);
            secondary_stats.critical_hit_chance = SecondaryStatsRow::compute_critical_hit_chance(
                res.level,
                primary_stats.ferocity,
                0.,
            );
            SecondaryStatsRow::update_from_self(secondary_stats, ctx);
        }
    }
}

#[spacetimedb::view(name = level_view, public)]
pub fn level_view(ctx: &ViewContext) -> Vec<LevelRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementStateRow::by_cell_id(ctx, cell_id))
        .filter_map(|ms| {
            LevelRow::find(ctx, ms.actor_id).map(|row| LevelRow {
                actor_id: ms.actor_id,
                level: row.level,
            })
        })
        .collect()
}
