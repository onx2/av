use crate::{level_tbl, EXPERIENCE_PER_LEVEL};
use shared::ActorId;
use spacetimedb::{table, ReducerContext, Table};

/// The amount of experience this person has accumulated
#[table(name = experience_tbl)]
pub struct ExperienceRow {
    #[primary_key]
    pub actor_id: ActorId,

    pub xp: u32,
}
impl ExperienceRow {
    pub fn add_exp(self, ctx: &ReducerContext, amount: u32) {
        let new_exp = self.xp.saturating_add(amount);
        self.update(ctx, new_exp);
    }

    pub fn sub_exp(self, ctx: &ReducerContext, amount: u32) {
        let new_exp = self.xp.saturating_sub(amount);
        self.update(ctx, new_exp);
    }

    fn update(mut self, ctx: &ReducerContext, new_exp: u32) {
        let Some(level_row) = ctx.db.level_tbl().actor_id().find(self.actor_id) else {
            return;
        };
        let new_level = ExperienceRow::level_from_xp(new_exp);
        self.xp = new_exp;
        ctx.db.experience_tbl().actor_id().update(self);

        if new_level > level_row.level {
            level_row.update(ctx, new_level);
        }
    }

    pub fn level_from_xp(xp: u32) -> u8 {
        EXPERIENCE_PER_LEVEL.partition_point(|&req| req <= xp) as u8
    }

    pub fn find(ctx: &ReducerContext, actor_id: ActorId) -> Option<Self> {
        ctx.db.experience_tbl().actor_id().find(actor_id)
    }

    pub fn insert(ctx: &ReducerContext, actor_id: ActorId, xp: u32) {
        ctx.db.experience_tbl().insert(Self { actor_id, xp });
    }

    pub fn delete(self, ctx: &ReducerContext) {
        ctx.db.experience_tbl().delete(self);
    }
}
