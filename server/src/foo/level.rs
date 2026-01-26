use shared::Owner;
use spacetimedb::{table, SpacetimeType};

/// The amount of progression this person has accumulated
#[table(name = level_tbl)]
pub struct Level {
    #[primary_key]
    pub owner: Owner,

    pub data: LevelData,

    #[index(btree)]
    pub cell_id: u32,
}
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelData {
    pub level: u8,
}

impl Default for LevelData {
    fn default() -> Self {
        Self { level: 1 }
    }
}
crate::impl_data_table!(table_handle = level_tbl, row = Level, data = LevelData);
