use super::{Quat, Vec3};
use nalgebra::Isometry3;
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table};

/// Ephemeral
///
/// The storage for various objects' transform data
#[table(name=transform_tbl)]
pub struct Transform {
    #[primary_key]
    pub owner: Owner,

    pub data: TransformData,
}

impl Transform {
    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.transform_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: TransformData) {
        ctx.db.transform_tbl().insert(Self { owner, data });
    }
    pub fn update(&self, ctx: &ReducerContext, data: TransformData) {
        ctx.db.transform_tbl().owner().update(Self {
            owner: self.owner,
            data,
        });
    }
}
#[derive(SpacetimeType, Debug, Default, PartialEq, Clone, Copy)]
pub struct TransformData {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl From<TransformData> for Isometry3<f32> {
    fn from(v: TransformData) -> Self {
        Self::from_parts(v.translation.into(), v.rotation.into())
    }
}
