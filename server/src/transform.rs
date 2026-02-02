use crate::{get_view_aoi_block, MovementStateRow};

use super::{Quat, Vec3};
use nalgebra::Isometry3;
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table, ViewContext};

/// Ephemeral
///
/// The storage for various objects' transform data
#[table(name=transform_tbl)]
pub struct TransformRow {
    #[primary_key]
    pub owner: Owner,

    pub data: TransformData,
}

impl TransformRow {
    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.transform_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: TransformData) {
        ctx.db.transform_tbl().insert(Self { owner, data });
    }
    /// Updates from given self, caller should have updated the state with the latest values.
    pub fn update_from_self(self, ctx: &ReducerContext) {
        ctx.db.transform_tbl().owner().update(self);
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

    // TODO: this doesn't need to be stored as a quat... yaw is fine.
    // We actually only need a u16 here too, quantizing to 2bytes from
    // the 16bytes really saves money
    pub rotation: Quat,
}

impl From<TransformData> for Isometry3<f32> {
    fn from(v: TransformData) -> Self {
        Self::from_parts(v.translation.into(), v.rotation.into())
    }
}

/// Finds the active character for all things within the AOI.
/// Primary key of `Identity`
#[spacetimedb::view(name = transform_view, public)]
pub fn transform_view(ctx: &ViewContext) -> Vec<TransformRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementStateRow::by_cell_id(ctx, cell_id))
        .filter_map(|ms| ctx.db.transform_tbl().owner().find(&ms.owner))
        .collect()
}
