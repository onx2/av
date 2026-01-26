use super::{ComputedStat, Stat};
use crate::foo::get_computed_stat_view;
use shared::Owner;
use spacetimedb::{LocalReadOnly, ViewContext};

pub struct CriticalHitChance;
impl ComputedStat for CriticalHitChance {
    type Output = Stat<f32>;
    fn compute(_db: &LocalReadOnly, _owner: Owner) -> Option<Self::Output> {
        Some(Stat { value: 5.0 })
    }
}
#[spacetimedb::view(name = critical_chance_view, public)]
pub fn critical_chance_view(ctx: &ViewContext) -> Vec<<CriticalHitChance as ComputedStat>::Output> {
    get_computed_stat_view::<CriticalHitChance>(ctx)
}
