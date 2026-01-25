use super::{Quat, Vec3};
use shared::owner::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType};

/// Ephemeral
///
/// The storage for various objects' transform data
#[table(name=transform_tbl)]
pub struct Transform {
    #[primary_key]
    pub owner: Owner,

    pub data: TransformData,
}

#[derive(SpacetimeType, Debug, Default, PartialEq, Clone, Copy)]
pub struct TransformData {
    pub translation: Vec3,
    pub rotation: Quat,
}
