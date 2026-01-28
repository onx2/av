use super::ComputedStat;
use crate::foo::get_computed_stat_view;
use shared::Owner;
use spacetimedb::{LocalReadOnly, SpacetimeType, ViewContext};

#[derive(SpacetimeType, Debug, Default)]
pub struct CriticalHitChance {
    pub owner: Owner,
    pub value: f32,
}
impl ComputedStat for CriticalHitChance {
    type Output = CriticalHitChance;
    fn compute(_db: &LocalReadOnly, owner: Owner) -> Option<Self::Output> {
        Some(CriticalHitChance { owner, value: 5.0 })
    }
}

/// Finds the critical hit chance stat for all actors within the AOI.
#[spacetimedb::view(name = critical_chance_view, public)]
pub fn critical_chance_view(ctx: &ViewContext) -> Vec<CriticalHitChance> {
    get_computed_stat_view::<CriticalHitChance>(ctx)
}
