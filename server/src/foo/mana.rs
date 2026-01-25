use shared::owner::Owner;
use spacetimedb::{table, SpacetimeType};

/// **Ephemeral**
#[table(name=mana_tbl)]
pub struct Mana {
    #[primary_key]
    pub owner: Owner,

    pub data: ManaData,
}
#[derive(SpacetimeType, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ManaData {
    pub current: u16,
    pub max: u16,
}

impl ManaData {
    pub fn new(max: u16) -> Self {
        Self { current: max, max }
    }
}
