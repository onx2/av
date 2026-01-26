use super::{Quat, Vec3};
use shared::Owner;
use spacetimedb::{table, SpacetimeType};

/// Ephemeral
///
/// The storage for various objects' transform data
#[table(name=transform_tbl)]
pub struct Transform {
    #[primary_key]
    pub owner: Owner,

    pub data: TransformData,

    #[index(btree)]
    pub cell_id: u32,
}
#[derive(SpacetimeType, Debug, Default, PartialEq, Clone, Copy)]
pub struct TransformData {
    pub translation: Vec3,
    pub rotation: Quat,
}
crate::impl_data_table!(
    table_handle = transform_tbl,
    row = Transform,
    data = TransformData
);
