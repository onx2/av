use shared::Owner;
use spacetimedb::table;

/// **Ephemeral**
#[table(name=health_tbl)]
pub struct Health {
    #[primary_key]
    pub owner: Owner,

    pub data: HealthData,
}
crate::bounded_stat!(HealthData, u16);
crate::impl_data_table!(table_handle = health_tbl, row = Health, data = HealthData);
