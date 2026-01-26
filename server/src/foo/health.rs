use shared::Owner;
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
crate::impl_data_table!(table_handle = health_tbl, row = Health, data = HealthData);

impl HealthData {
    pub fn new(max: u16) -> Self {
        Self { current: max, max }
    }

    pub fn add(&mut self, amount: u16) {
        self.current = self.current.saturating_add(amount).min(self.max);
    }

    pub fn sub(&mut self, amount: u16) {
        self.current = self.current.saturating_sub(amount);
    }
}
