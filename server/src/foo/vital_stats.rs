use shared::owner::Owner;
use spacetimedb::{table, SpacetimeType};

/// **Ephemeral**
///
/// The storage for Vitals data like "health". Intentionally isolated and slim because it is considered "hot", meaning it is
/// expected to change more frequently than other types of data.
#[table(name=vital_stats_tbl)]
pub struct VitalStats {
    #[primary_key]
    pub owner: Owner,

    pub data: VitalStatsData,
}
#[derive(SpacetimeType, Debug, PartialEq, Clone)]
pub struct VitalStatsData {
    pub health: u16,
    pub mana: u16,
    pub stamina: u16,
}
