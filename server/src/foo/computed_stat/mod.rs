// ------------------------------------------------------------------
// TODO: THIS IS HANDY FOR SPECIFYING THE FORMULA FOR A GIVEN STAT
// BUT IT IS NOT OPTIMIZED FOR PERFORMANCE OR COST. IDEALLY THESE
// STATS HAVE A SHARED FUNCTION TO COMPUTE THEM AND WHEN ONE OF THE
// INPUT VALUES CHANGES, THE STAT IS RECOMPUTED INLINE.
// ------------------------------------------------------------------
mod critical_hit_chance;
mod movement_speed;

use crate::foo::{active_character_tbl__view, movement_state_tbl__view};
pub use critical_hit_chance::*;
pub use movement_speed::*;
use shared::{utils::get_aoi_block, Owner};
use spacetimedb::{DbContext, LocalReadOnly, SpacetimeType, ViewContext};

pub trait ComputedStat {
    type Output: SpacetimeType;
    fn compute(db: &LocalReadOnly, owner: Owner) -> Option<Self::Output>;
}

/// Generic “computed stat view” helper: for the sender, compute `S` for every actor in AOI.
///
/// For example, we could find all the [CriticalHitChance] values for actors in the AOI:
/// ```rust
/// pub fn critical_chance_view(ctx: &ViewContext) -> Vec<CriticalHitChance> {
///     get_computed_stat_view::<CriticalHitChance>(ctx)
/// }
/// ```
pub fn get_computed_stat_aoi_view<S>(ctx: &ViewContext) -> Vec<S::Output>
where
    S: ComputedStat,
    S::Output: Default,
{
    let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
        return vec![];
    };
    let Some(cell_id) = ctx
        .db
        .movement_state_tbl()
        .owner()
        .find(&active_character.owner)
        .map(|row| row.cell_id)
    else {
        return vec![];
    };

    get_aoi_block(cell_id)
        .into_iter()
        .flat_map(|cell_id| ctx.db.movement_state_tbl().cell_id().filter(cell_id))
        .map(|ms| S::compute(ctx.db(), ms.owner).unwrap_or_default())
        .collect()
}
