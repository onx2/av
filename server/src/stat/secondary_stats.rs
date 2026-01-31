use crate::{get_view_aoi_block, MovementState};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table, ViewContext};

#[table(name=secondary_stats_tbl)]
pub struct SecondaryStats {
    #[primary_key]
    pub owner: Owner,

    pub data: SecondaryStatsData,
}

impl SecondaryStats {
    pub fn find(ctx: &ViewContext, owner: Owner) -> Option<Self> {
        ctx.db.secondary_stats_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: SecondaryStatsData) {
        ctx.db.secondary_stats_tbl().insert(Self { owner, data });
    }
}

#[derive(SpacetimeType, Debug, PartialEq, Clone, Copy)]
pub struct SecondaryStatsData {
    pub movement_speed: f32,
    pub critical_hit_chance: f32,
    // armor
    // attack_speed
}

impl SecondaryStatsData {
    /// Movement speed is determined by level, buffs, and gear only.
    ///
    /// Note: Bonus values should be passed in as decimal percentages (normalized between 0 and 1)
    /// and the multiplier will be computed based on that.
    ///
    /// TODO: implement buffs and gear
    pub fn compute_movement_speed(level: u8, gear: f32, buff: f32, debuff: f32) -> f32 {
        let base_speed = 4.0;
        let level_bonus = level as f32 * 0.04; // Max lvl 50 -> 50 * 0.04 = 2 m/s bonus
        let gear_multiplier = 1. + gear;
        let buff_multiplier = 1. + buff;
        let debuff_multiplier = 1. - debuff;
        // Compute the speed from multipliers but hard cap at 10m/s
        // Ideally buffs grant up to an additional 30% movement speed and gear +20%
        // Meaning (4 + 2) * (1 + 0.2) * (1 + 0.1) -> ~9.36, so the 10m/s cap is just a safety net
        ((base_speed + level_bonus) * gear_multiplier * buff_multiplier * debuff_multiplier)
            .min(10.0)
    }

    /// Critical hit chance is determined by level, ferocity (primary stat), and gear
    ///
    /// Note: Bonus values should be passed in as decimal percentages (normalized between 0 and 1)
    /// and the multiplier will be computed based on that.
    ///
    /// TODO: implement gear
    pub fn compute_critical_hit_chance(level: u8, ferocity: u8, gear: f32) -> f32 {
        let base_speed = 5.0;
        let ferocity_bonus = ferocity as f32 * 0.075;
        let level_bonus = level as f32 * 0.01;
        let gear_multiplier = 1. + gear;
        // Max critical hit chance of 50% seems reasonable for now... tbd
        (base_speed * (1. + ferocity_bonus + level_bonus) * gear_multiplier).min(50.0)
    }
}

#[derive(SpacetimeType, Debug)]
pub struct SecondaryStatsRow {
    pub owner: Owner,
    pub data: SecondaryStatsData,
}
/// Finds the secondary stats for all actors within the AOI.
/// Primary key of `Owner`
#[spacetimedb::view(name = secondary_stats_view, public)]
pub fn secondary_stats_view(ctx: &ViewContext) -> Vec<SecondaryStatsRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementState::by_cell_id(ctx, cell_id))
        .filter_map(|ms| {
            SecondaryStats::find(ctx, ms.owner).map(|row| SecondaryStatsRow {
                owner: ms.owner,
                data: row.data,
            })
        })
        .collect()
}
