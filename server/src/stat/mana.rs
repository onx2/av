use crate::{get_view_aoi_block, MovementState};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table, ViewContext};

/// **Ephemeral**
#[table(name=mana_tbl)]
pub struct Mana {
    #[primary_key]
    pub owner: Owner,

    pub data: ManaData,
}

impl Mana {
    fn clamp(&mut self) {
        self.data.current = self.data.current.min(self.data.max);
    }

    pub fn insert(ctx: &ReducerContext, owner: Owner, data: ManaData) {
        ctx.db.mana_tbl().insert(Self { owner, data });
    }

    pub fn find(ctx: &ViewContext, owner: Owner) -> Option<Self> {
        ctx.db.mana_tbl().owner().find(owner)
    }

    pub fn set_current(mut self, ctx: &ReducerContext, value: u16) {
        self.data.current = value;
        self.clamp();
        ctx.db.mana_tbl().owner().update(self);
    }

    pub fn set_max(mut self, ctx: &ReducerContext, value: u16) {
        self.data.max = value;
        self.clamp();

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

    /// Formula to compute the maximum mana based on level and intelligence.
    /// TBD on if this should exist in the shared crate
    pub fn compute_max(level: u8, intelligence: u8) -> u16 {
        let base: u16 = 200;

        // Clamped to max values for computation
        let intelligence = (intelligence as u16).min(60);
        let level = (level as u16).min(50);

        let growth = level.pow(2) * 5; // 50 * 50 * 5 = 12500
        let bonus = intelligence * level * 9; // 60 * 50 * 9 = 27000
        base.saturating_add(growth).saturating_add(bonus)
    }
}

#[derive(SpacetimeType, Debug)]
pub struct ManaRow {
    pub owner: Owner,
    pub data: ManaData,
}
/// Finds the health for all things within the AOI.
/// Primary key of `Owner`
#[spacetimedb::view(name = mana_view, public)]
pub fn mana_view(ctx: &ViewContext) -> Vec<ManaRow> {
    let Some(cell_block) = get_view_aoi_block(ctx) else {
        return vec![];
    };

    cell_block
        .flat_map(|cell_id| MovementState::by_cell_id(ctx, cell_id))
        .filter_map(|ms| {
            Mana::find(ctx, ms.owner).map(|row| ManaRow {
                owner: ms.owner,
                data: row.data,
            })
        })
        .collect()
}
