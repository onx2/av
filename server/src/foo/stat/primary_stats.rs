use crate::foo::{secondary_stats_tbl, Level, SecondaryStatsData};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table};

/// Ephemeral
///
/// The primary driving factors for other aspects of gameplay (secondary stats, damage, etc...)
#[table(name=primary_stats_tbl)]
pub struct PrimaryStats {
    #[primary_key]
    pub owner: Owner,

    pub data: PrimaryStatsData,
}

impl PrimaryStats {
    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.primary_stats_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: PrimaryStatsData) {
        ctx.db.primary_stats_tbl().insert(Self { owner, data });
    }
    pub fn update(&self, ctx: &ReducerContext, data: PrimaryStatsData) {
        let original_ferocity = self.data.ferocity;
        let primary_stats = ctx.db.primary_stats_tbl().owner().update(Self {
            owner: self.owner,
            data,
        });

        // Update derived stats when the base values change. Right now only ferocity is used.
        if original_ferocity == data.ferocity {
            return;
        }

        if let Some(mut secondary_stats) = ctx.db.secondary_stats_tbl().owner().find(self.owner) {
            let Some(level) = Level::find(ctx, self.owner).map(|r| r.data.level) else {
                log::error!("Unable to find level for owner: {:?}", self.owner);
                return;
            };

            secondary_stats.data.critical_hit_chance =
                SecondaryStatsData::compute_critical_hit_chance(
                    level,
                    primary_stats.data.ferocity,
                    0.,
                );
            ctx.db.secondary_stats_tbl().owner().update(secondary_stats);
        }
    }
}

#[derive(SpacetimeType, Debug, PartialEq, Clone, Copy)]
pub struct PrimaryStatsData {
    /// Used in part to determine the critical chance of attacks and abilities
    pub ferocity: u8,

    /// Used in part to determine the maximum health capacity
    pub fortitude: u8,

    /// Used in part to determine the maximum mana capacity
    /// This is the overall understanding of The Veil and its innerworkings, increasing the available.
    pub intellect: u8,

    /// Used in part to determine the power of abilities
    /// This is proficiency in using The Veil and thus increases the damage or healing capabilities.
    pub acuity: u8,

    /// The points that are currently available for the owner to place into primary stats
    /// Players are granted 1 per level, but 3 per tier level (10,20,30,40,50)
    pub available_points: u8,
}

impl Default for PrimaryStatsData {
    fn default() -> Self {
        Self {
            ferocity: Self::MIN_STAT,
            fortitude: Self::MIN_STAT,
            intellect: Self::MIN_STAT,
            acuity: Self::MIN_STAT,
            available_points: 0,
        }
    }
}

impl PrimaryStatsData {
    const CREATION_POINTS: u8 = 15;
    const MIN_STAT: u8 = 10;
    const MAX_STAT: u8 = 100;

    pub fn validate(&self) -> bool {
        let stats = [self.ferocity, self.fortitude, self.intellect, self.acuity];

        // Per-stat bounds.
        if !stats
            .iter()
            .all(|&v| v >= Self::MIN_STAT && v <= Self::MAX_STAT)
        {
            return false;
        }

        // Total cap: (# of stats * MIN_STAT) + CREATION_POINTS
        // Use u16 to avoid any chance of overflow during accumulation.
        let max_total =
            (Self::MIN_STAT as u16) * (stats.len() as u16) + (Self::CREATION_POINTS as u16);
        let total = stats.iter().map(|&v| v as u16).sum::<u16>();

        total == max_total
    }
}
