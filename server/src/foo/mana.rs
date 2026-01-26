use shared::Owner;
use spacetimedb::{table, SpacetimeType};

/// **Ephemeral**
#[table(name=mana_tbl)]
pub struct Mana {
    #[primary_key]
    pub owner: Owner,

    pub data: ManaData,

    #[index(btree)]
    pub cell_id: u32,
}
#[derive(SpacetimeType, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ManaData {
    pub current: u16,
    pub max: u16,
}
crate::impl_data_table!(table_handle = mana_tbl, row = Mana, data = ManaData);

impl ManaData {
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
