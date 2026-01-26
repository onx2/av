use super::{ComputedStat, Stat};
use crate::foo::active_character_tbl__view;
use shared::Owner;
use spacetimedb::{DbContext, LocalReadOnly, ViewContext};

pub struct CriticalHitChance;
impl ComputedStat for CriticalHitChance {
    type Output = Stat<f32>;
    fn compute(_db: &LocalReadOnly, _owner: Owner) -> Option<Self::Output> {
        Some(Stat { value: 5.0 })
    }
}
#[spacetimedb::view(name = critical_chance_view, public)]
pub fn critical_chance_view(
    ctx: &ViewContext,
) -> Option<<CriticalHitChance as ComputedStat>::Output> {
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return None;
    };

    CriticalHitChance::compute(ctx.db(), active_character.owner)
}
