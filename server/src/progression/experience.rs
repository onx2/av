use crate::{level_tbl, EXPERIENCE_PER_LEVEL};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table};

/// The amount of experience this person has accumulated
#[table(name = experience_tbl)]
pub struct ExperienceRow {
    #[primary_key]
    pub owner: Owner,

    pub data: ExperienceData,
}
impl ExperienceRow {
    pub fn add_exp(self, ctx: &ReducerContext, amount: u32) {
        let new_exp = self.data.experience.saturating_add(amount);
        self.update(ctx, new_exp);
    }

    pub fn sub_exp(self, ctx: &ReducerContext, amount: u32) {
        let new_exp = self.data.experience.saturating_sub(amount);
        self.update(ctx, new_exp);
    }

    fn update(mut self, ctx: &ReducerContext, new_exp: u32) {
        let Some(level_row) = ctx.db.level_tbl().owner().find(self.owner) else {
            return;
        };
        let new_level = ExperienceRow::level_from_xp(new_exp);
        self.data.experience = new_exp;
        ctx.db.experience_tbl().owner().update(self);

        if new_level > level_row.data.level {
            level_row.update(ctx, new_level);
        }
    }

    pub fn level_from_xp(xp: u32) -> u8 {
        EXPERIENCE_PER_LEVEL.partition_point(|&req| req <= xp) as u8
    }

    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.experience_tbl().owner().find(owner)
    }

    pub fn insert(ctx: &ReducerContext, owner: Owner, data: ExperienceData) {
        ctx.db.experience_tbl().insert(Self { owner, data });
    }

    pub fn delete(self, ctx: &ReducerContext) {
        ctx.db.experience_tbl().delete(self);
    }
}
#[derive(SpacetimeType, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExperienceData {
    pub experience: u32,
}
