use shared::Owner;
use spacetimedb::table;

/// **Ephemeral**
#[table(name=mana_tbl)]
pub struct Mana {
    #[primary_key]
    pub owner: Owner,

    pub data: ManaData,
}
crate::bounded_stat!(ManaData, u16);
crate::impl_data_table!(table_handle = mana_tbl, row = Mana, data = ManaData);
