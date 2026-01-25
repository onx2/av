use shared::owner::Owner;
use spacetimedb::{table, SpacetimeType};

/// Ephemeral
///
/// The derived / computed stats for an owner, based on various things like PrimaryStats, equipment, perks, spells, etc...
#[table(name=secondary_stats_tbl)]
pub struct SecondaryStats {
    #[primary_key]
    pub owner: Owner,

    pub data: SecondaryStatsData,
}
#[derive(SpacetimeType, Debug, PartialEq, Clone, Copy)]
pub struct SecondaryStatsData {
    pub movement_speed: f32,
}

impl Default for SecondaryStatsData {
    fn default() -> Self {
        Self {
            movement_speed: 5.0,
        }
    }
}
