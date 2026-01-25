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

#[derive(SpacetimeType, Debug, PartialEq, Clone, Copy)]
pub struct PrimaryStatsData {
    pub strength: u8,
    pub dexterity: u8,
    pub fortitude: u8,
    pub intelligence: u8,
    pub piety: u8,
}

impl Default for PrimaryStatsData {
    fn default() -> Self {
        Self {
            strength: Self::MIN_STAT,
            dexterity: Self::MIN_STAT,
            fortitude: Self::MIN_STAT,
            intelligence: Self::MIN_STAT,
            piety: Self::MIN_STAT,
        }
    }
}

impl PrimaryStatsData {
    const MIN_STAT: u8 = 10;
    const MAX_STAT: u8 = 100;

    pub fn validate(&self) -> bool {
        [
            self.strength,
            self.dexterity,
            self.fortitude,
            self.intelligence,
            self.piety,
        ]
        .iter()
        .all(|&v| v >= Self::MIN_STAT && v <= Self::MAX_STAT)
    }
}
