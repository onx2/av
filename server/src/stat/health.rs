use crate::{get_view_aoi_block, MovementStateRow};
use shared::ActorId;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table, ViewContext};

/// **Ephemeral**
#[table(name=health_tbl)]
pub struct HealthRow {
    #[primary_key]
    pub actor_id: ActorId,

    pub data: HealthData,

    /// Indexed lookup for "is current health at max?"
    #[index(btree)]
    pub is_full: bool,
}

impl HealthRow {
    fn clamp(&mut self) {
        self.data.current = self.data.current.min(self.data.max);
    }

    pub fn insert(ctx: &ReducerContext, actor_id: ActorId, data: HealthData) {
        let current = data.current.min(data.max);
        ctx.db.health_tbl().insert(Self {
            actor_id,
            is_full: current == data.max,
            data: HealthData { current, ..data },
        });
    }

    pub fn find(ctx: &ViewContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.health_tbl().actor_id().find(actor_id)
    }

    /// Adds to the current value, clamping and computing is_full
    pub fn add(mut self, ctx: &ReducerContext, amount: u16) {
        if amount == 0 {
            return;
        }
        self.data.current = self.data.current.saturating_add(amount);
        self.clamp();
        self.is_full = self.data.current == self.data.max;
        ctx.db.health_tbl().actor_id().update(self);
    }

    /// Subtracts from the current value, clamping and computing is_full
    pub fn sub(mut self, ctx: &ReducerContext, amount: u16) {
        if amount == 0 {
            return;
        }
        self.data.current = self.data.current.saturating_sub(amount);
        self.clamp();
        self.is_full = self.data.current == self.data.max;
        ctx.db.health_tbl().actor_id().update(self);
    }

    /// Sets the current value, clamping to max and computing is_full
    pub fn set_current(mut self, ctx: &ReducerContext, value: u16) {
        if value == self.data.current {
            return;
        }
        self.data.current = value;
        self.clamp();
        self.is_full = self.data.current == self.data.max;
        ctx.db.health_tbl().actor_id().update(self);
    }

    /// Sets the max value, clamping current and computing is_full
    pub fn set_max(mut self, ctx: &ReducerContext, value: u16) {
        if value == self.data.max {
            return;
        }
        self.data.max = value;
        self.clamp();
        self.is_full = self.data.current == self.data.max;
        ctx.db.health_tbl().actor_id().update(self);
    }
}

#[derive(SpacetimeType, Debug, PartialEq, Eq, Clone, Copy)]
pub struct HealthData {
    pub current: u16,
    pub max: u16,
}

impl HealthData {
    pub fn new(max: u16) -> Self {
        Self { current: max, max }
    }

    /// Formula to compute the maximum health based on level and fortitude.
    /// TBD on if this should exist in the shared crate
    pub fn compute_max(level: u8, fortitude: u8) -> u16 {
        let base: u16 = 200;

        // Clamped to max values for computation
        let fortitude = (fortitude as u16).min(60);
        let level = (level as u16).min(50);
        let growth = level.pow(2) * 5; // 50 * 50 * 5 = 12500
        let bonus = fortitude * level * 9; // 60 * 50 * 9 = 27000
        base.saturating_add(growth).saturating_add(bonus)
    }
}

/// Finds the health for all things within the AOI.
/// Primary key of `ActorId`
#[spacetimedb::view(name = health_view, public)]
pub fn health_view(ctx: &ViewContext) -> Vec<HealthRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementStateRow::by_cell_id(ctx, cell_id))
        .filter_map(|ms| {
            HealthRow::find(ctx, ms.actor_id).map(|row| HealthRow {
                actor_id: ms.actor_id,
                data: row.data,
                is_full: row.is_full,
            })
        })
        .collect()
}
