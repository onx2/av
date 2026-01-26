mod critical_hit_chance;
mod movement_speed;

use crate::foo::{active_character_tbl__view, actor_tbl__view};
pub use critical_hit_chance::*;
pub use movement_speed::*;
use shared::{utils::get_aoi_block, Owner};
use spacetimedb::{DbContext, LocalReadOnly, SpacetimeType, ViewContext};
use std::ops::Deref;

/// A small, reusable spacetime payload for "scalar" stats.
#[derive(SpacetimeType, Debug, Default)]
pub struct Stat<T: Copy + Default> {
    pub value: T,
}
impl<T: Copy + Default> Stat<T> {
    pub fn new(value: T) -> Self {
        Stat { value }
    }
}
impl<T: Copy + Default> Deref for Stat<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub trait ComputedStat {
    type Output: SpacetimeType;
    fn compute(db: &LocalReadOnly, owner: Owner) -> Option<Self::Output>;
}

/// Generic “computed stat view” helper: for the sender, compute `S` for every actor in AOI.
///
/// This is intended to compile down to essentially the same code as writing the iterator chain
/// inline in every view (monomorphized generics).
///
/// So for example, we could find all the [CriticalHitChance] values for actors in the AOI:
///
/// ```rust
/// pub fn critical_chance_view(ctx: &ViewContext) -> Vec<CriticalHitChance> {
///     get_computed_stat_view::<CriticalHitChance>(ctx)
/// }
/// ```
pub fn get_computed_stat_view<S>(ctx: &ViewContext) -> Vec<S::Output>
where
    S: ComputedStat,
    S::Output: Default,
{
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return vec![];
    };
    let Some(actor) = ctx.db.actor_tbl().owner().find(&active_character.owner) else {
        return vec![];
    };

    get_aoi_block(actor.cell_id)
        .into_iter()
        .flat_map(|cell_id| ctx.db.actor_tbl().cell_id().filter(cell_id))
        .map(|a| S::compute(ctx.db(), a.owner).unwrap_or_default())
        .collect()
}
