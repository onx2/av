use crate::foo::{level_tbl, EXPERIENCE_PER_LEVEL};
use shared::Owner;
use spacetimedb::{table, ReducerContext, SpacetimeType, Table};

/// The amount of experience this person has accumulated
#[table(name = experience_tbl)]
pub struct Experience {
    #[primary_key]
    pub owner: Owner,

    pub data: ExperienceData,
}
impl Experience {
    pub fn level_from_xp(xp: u32) -> u8 {
        EXPERIENCE_PER_LEVEL.partition_point(|&req| req <= xp) as u8
    }

    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.experience_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: ExperienceData) {
        ctx.db.experience_tbl().insert(Self { owner, data });
    }
    pub fn add_exp(mut self, ctx: &ReducerContext, amount: u32) {
        let Some(mut level_row) = ctx.db.level_tbl().owner().find(self.owner) else {
            return;
        };

        let new_exp = self.data.experience.saturating_add(amount);
        let new_level = Experience::level_from_xp(new_exp);

        self.data.experience = new_exp;
        ctx.db.experience_tbl().owner().update(self);

        if new_level > level_row.data.level {
            level_row.data.level = new_level;
            ctx.db.level_tbl().owner().update(level_row);
        }
    }
}
#[derive(SpacetimeType, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExperienceData {
    pub experience: u32,
}
// crate::impl_data_table!(
//     table_handle = experience_tbl,
//     row = Experience,
//     data = ExperienceData
// );
