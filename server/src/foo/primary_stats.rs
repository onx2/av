use shared::owner::Owner;
use spacetimedb::{table, SpacetimeType};

/// Ephemeral
///
/// The primary driving factors for other aspects of gameplay (secondary stats, damage, etc...)
#[table(name=primary_stats_tbl)]
pub struct PrimaryStats {
    #[primary_key]
    pub owner: Owner,

    pub data: PrimaryStatsData,
}

#[derive(SpacetimeType, Debug, PartialEq, Clone)]
pub struct PrimaryStatsData {
    pub strength: u8,
    pub dexterity: u8,
    pub fortitude: u8,
    pub intelligence: u8,
    pub piety: u8,
}
