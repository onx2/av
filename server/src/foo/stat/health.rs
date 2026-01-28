use shared::Owner;
use spacetimedb::{table, LocalReadOnly, ReducerContext, SpacetimeType};

/// **Ephemeral**
#[table(name=health_tbl)]
pub struct Health {
    #[primary_key]
    pub owner: Owner,

    pub data: HealthData,
}
crate::impl_data_table!(table_handle = health_tbl, row = Health, data = HealthData);

impl Health {
    fn clamp(&mut self) {
        if self.data.current > self.data.max {
            self.data.current = self.data.max;
        }
    }

    pub fn find(db: &LocalReadOnly, owner: Owner) -> Option<Self> {
        db.health_tbl().owner().find(owner)
    }

    pub fn set_current(mut self, ctx: &ReducerContext, value: u16) {
        self.data.current = value;
        self.clamp();
        ctx.db.health_tbl().owner().update(self);
    }

    pub fn set_max(mut self, ctx: &ReducerContext, value: u16) {
        self.data.max = value;
        self.clamp();

        ctx.db.health_tbl().owner().update(self);
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
