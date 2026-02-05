use crate::{get_view_aoi_block, MovementStateRow, Vec3};
use nalgebra::{Isometry3, UnitQuaternion, Vector3};
use shared::ActorId;
use spacetimedb::{table, ReducerContext, Table, ViewContext};

/// Ephemeral
///
/// The storage for various objects' transform data
#[table(name=transform_tbl)]
pub struct TransformRow {
    #[primary_key]
    pub actor_id: ActorId,

    // [OPTIMIZE TODO]
    // This can probably be removed and computed on the client
    // We'd really only need yaw on the server during event-driven things...
    // keeping for now though just in case.
    pub yaw: f32,

    pub translation: Vec3,
}

impl TransformRow {
    pub fn find(ctx: &ReducerContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.transform_tbl().actor_id().find(actor_id)
    }
    pub fn insert(ctx: &ReducerContext, actor_id: ActorId, translation: Vec3, yaw: f32) {
        ctx.db.transform_tbl().insert(Self {
            actor_id,
            translation,
            yaw,
        });
    }
    /// Updates from given self, caller should have updated the state with the latest values.
    pub fn update_from_self(self, ctx: &ReducerContext) {
        ctx.db.transform_tbl().actor_id().update(self);
    }
    pub fn update(&self, ctx: &ReducerContext, translation: Vec3, yaw: f32) {
        ctx.db.transform_tbl().actor_id().update(Self {
            actor_id: self.actor_id,
            translation,
            yaw,
        });
    }
}

pub fn to_isometry3(row: &TransformRow) -> Isometry3<f32> {
    let rotation = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), row.yaw);
    Isometry3::from_parts(row.translation.into(), rotation)
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
        .filter_map(|ms| ctx.db.transform_tbl().actor_id().find(&ms.actor_id))
        .collect()
}
