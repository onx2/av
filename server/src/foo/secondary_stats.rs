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
#[derive(SpacetimeType, Debug, PartialEq, Clone)]
pub struct SecondaryStatsData {
    pub max_health: u16,
    pub max_mana: u16,
    pub max_stamina: u16,
    pub movement_speed: f32,
}
