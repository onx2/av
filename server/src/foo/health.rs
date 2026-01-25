use shared::owner::Owner;
use spacetimedb::{table, SpacetimeType};

/// **Ephemeral**
#[table(name=health_tbl)]
pub struct Health {
    #[primary_key]
    pub owner: Owner,

    pub data: HealthData,
}
#[derive(SpacetimeType, Debug, PartialEq, Eq, Clone, Copy)]
pub struct HealthData {
    pub current: u16,
    pub max: u16,
}
impl HealthData {
    pub fn new(max: u16) -> Self {
        Self { current: max, max }
    }
}
