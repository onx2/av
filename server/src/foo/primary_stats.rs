use shared::Owner;
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
crate::impl_data_table!(
    table_handle = primary_stats_tbl,
    row = PrimaryStats,
    data = PrimaryStatsData
);

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
    const CREATION_POINTS: u8 = 15;
    const MIN_STAT: u8 = 10;
    const MAX_STAT: u8 = 100;

    pub fn validate(&self) -> bool {
        let stats = [
            self.strength,
            self.dexterity,
            self.fortitude,
            self.intelligence,
            self.piety,
        ];

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
