use crate::{get_view_aoi_block, MovementStateRow};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table, ViewContext};

/// **Ephemeral**
#[table(name=mana_tbl)]
pub struct ManaRow {
    #[primary_key]
    pub owner: Owner,

    /// Indexed lookup for "is current mana at max?"
    #[index(btree)]
    pub is_full: bool,

    pub data: ManaData,
}

impl ManaRow {
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: ManaData) {
        let current = data.current.min(data.max);
        ctx.db.mana_tbl().insert(Self {
            owner,
            is_full: current == data.max,
            data: ManaData { current, ..data },
        });
    }

    pub fn find(ctx: &ViewContext, owner: Owner) -> Option<Self> {
        ctx.db.mana_tbl().owner().find(owner)
    }

    /// Adds to the current value, clamping and computing is_full
    pub fn add(mut self, ctx: &ReducerContext, amount: u16) {
        if amount == 0 || self.is_full {
            return;
        }

        self.data.current = self.data.current.saturating_add(amount).min(self.data.max);
        self.is_full = self.data.current == self.data.max;
        ctx.db.mana_tbl().owner().update(self);
    }

    /// Subtracts from the current value, clamping and computing is_full
    pub fn sub(mut self, ctx: &ReducerContext, amount: u16) {
        if amount == 0 || self.data.current == 0 {
            return;
        }
        self.data.current = self.data.current.saturating_sub(amount);
        self.is_full = self.data.current == self.data.max;
        ctx.db.mana_tbl().owner().update(self);
    }

    /// Sets the current value, clamping to max and computing is_full
    pub fn set_current(mut self, ctx: &ReducerContext, value: u16) {
        if value == self.data.current {
            return;
        }

        self.data.current = value.min(self.data.max);
        self.is_full = self.data.current == self.data.max;
        ctx.db.mana_tbl().owner().update(self);
    }

    /// Sets the max value, clamping current and computing is_full
    pub fn set_max(mut self, ctx: &ReducerContext, value: u16) {
        if value == self.data.max {
            return;
        }
        self.data.max = value;
        self.data.current = self.data.current.min(value);
        self.is_full = self.data.current == self.data.max;
        ctx.db.mana_tbl().owner().update(self);
    }
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

    /// Formula to compute the maximum mana based on level and intellect.
    /// TBD on if this should exist in the shared crate
    pub fn compute_max(level: u8, intellect: u8) -> u16 {
        let base: u16 = 200;

        // Clamped to max values for computation
        let intellect = (intellect as u16).min(60);
        let level = (level as u16).min(50);

        let growth = level.pow(2) * 5; // 50 * 50 * 5 = 12500
        let bonus = intellect * level * 9; // 60 * 50 * 9 = 27000
        base.saturating_add(growth).saturating_add(bonus)
    }
}

/// Finds the mana for all things within the AOI.
/// Primary key of `Owner`
#[spacetimedb::view(name = mana_view, public)]
pub fn mana_view(ctx: &ViewContext) -> Vec<ManaRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementStateRow::by_cell_id(ctx, cell_id))
        .filter_map(|ms| {
            ManaRow::find(ctx, ms.owner).map(|row| ManaRow {
                owner: ms.owner,
                data: row.data,
                is_full: row.is_full,
            })
        })
        .collect()
}
