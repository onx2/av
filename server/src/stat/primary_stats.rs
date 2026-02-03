use crate::{
    character_instance_tbl, character_instance_tbl__view, LevelData, LevelRow, SecondaryStatsData,
    SecondaryStatsRow,
};
use shared::ActorId;
use spacetimedb::{reducer, table, ReducerContext, SpacetimeType, Table, ViewContext};

/// Ephemeral
///
/// The primary driving factors for other aspects of gameplay (secondary stats, damage, etc...)
#[table(name=primary_stats_tbl)]
pub struct PrimaryStatsRow {
    #[primary_key]
    pub actor_id: ActorId,

    pub data: PrimaryStatsData,
}

impl PrimaryStatsRow {
    pub fn find(ctx: &ViewContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.primary_stats_tbl().actor_id().find(actor_id)
    }
    pub fn insert(ctx: &ReducerContext, actor_id: ActorId, data: PrimaryStatsData) {
        ctx.db.primary_stats_tbl().insert(Self { actor_id, data });
    }

    pub fn update(&self, ctx: &ReducerContext, data: PrimaryStatsData) {
        let original_ferocity = self.data.ferocity;
        let primary_stats = ctx.db.primary_stats_tbl().actor_id().update(Self {
            actor_id: self.actor_id,
            data,
        });

        // TODO: Update derived stats when the base values change. Right now only ferocity is used.
        if original_ferocity == data.ferocity {
            return;
        }

        let view_ctx = ctx.as_read_only();
        if let Some(mut secondary_stats) = SecondaryStatsRow::find(&view_ctx, self.actor_id) {
            let Some(level) = LevelRow::find(&view_ctx, self.actor_id).map(|r| r.data.level) else {
                log::error!("Unable to find level for actor: {:?}", self.actor_id);
                return;
            };

            secondary_stats.data.critical_hit_chance =
                SecondaryStatsData::compute_critical_hit_chance(
                    level,
                    primary_stats.data.ferocity,
                    0.,
                );
            secondary_stats.update_from_self(ctx);
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
    const MIN_STAT: u8 = 10;
    const MAX_STAT: u8 = 60;

    /// Determines if stats are within bounds of the available points, level, and and min/max
    pub fn validate(&self, level: u8) -> bool {
        let stats = [self.ferocity, self.fortitude, self.intellect, self.acuity];

        // Per-stat bounds, are we within the min and max?
        if !stats
            .iter()
            .all(|&v| v >= Self::MIN_STAT && v <= Self::MAX_STAT)
        {
            return false;
        }

        // Are we within the total cap for this level?
        let total: u8 = stats.iter().sum();
        if total > LevelData::points_for_level(level) {
            return false;
        }

        true
    }
}

/// Finds the primary stats for this actor.
///
/// Primary key of `Owner`
#[spacetimedb::view(name = primary_stats_view, public)]
pub fn primary_stats_view(ctx: &ViewContext) -> Option<PrimaryStatsData> {
    let Some(active_character) = ctx.db.character_instance_tbl().identity().find(&ctx.sender)
    else {
        return None;
    };

    PrimaryStatsRow::find(ctx, active_character.actor_id).map(|ps| ps.data)
}

#[derive(SpacetimeType)]
pub struct PlacePointsInput {
    pub new_ferocity: u8,
    pub new_fortitude: u8,
    pub new_intellect: u8,
    pub new_acuity: u8,
}

#[reducer]
pub fn place_points(ctx: &ReducerContext, input: PlacePointsInput) -> Result<(), String> {
    let view_ctx = ctx.as_read_only();
    let Some(active_character) = ctx
        .db
        .character_instance_tbl()
        .identity()
        .find(&view_ctx.sender)
    else {
        return Err("No active character found".to_string());
    };
    let Some(ps) = PrimaryStatsRow::find(&view_ctx, active_character.actor_id) else {
        return Err("No primary stats found".to_string());
    };

    // Each stat can only increase (never decrease).
    if input.new_ferocity < ps.data.ferocity
        || input.new_fortitude < ps.data.fortitude
        || input.new_intellect < ps.data.intellect
        || input.new_acuity < ps.data.acuity
    {
        return Err("Primary stats cannot be decreased".to_string());
    }

    // Prevent going over max
    if input.new_ferocity > PrimaryStatsData::MAX_STAT
        || input.new_fortitude > PrimaryStatsData::MAX_STAT
        || input.new_intellect > PrimaryStatsData::MAX_STAT
        || input.new_acuity > PrimaryStatsData::MAX_STAT
    {
        return Err("Primary stat exceeds maximum".to_string());
    }

    let current_total = ps.data.acuity as u16
        + ps.data.ferocity as u16
        + ps.data.fortitude as u16
        + ps.data.intellect as u16;
    let sent_total = input.new_acuity as u16
        + input.new_ferocity as u16
        + input.new_fortitude as u16
        + input.new_intellect as u16;

    let spent = (sent_total - current_total) as u8;
    if spent > ps.data.available_points {
        return Err("Not enough available points".to_string());
    }

    // Apply update and decrement remaining points by the amount spent.
    ps.update(
        ctx,
        PrimaryStatsData {
            ferocity: input.new_ferocity,
            fortitude: input.new_fortitude,
            intellect: input.new_intellect,
            acuity: input.new_acuity,
            available_points: (ps.data.available_points - spent) as u8,
        },
    );

    Ok(())
}
