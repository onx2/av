// use crate::foo::active_character_tbl__view;

// use super::ComputedStat;
// use shared::Owner;
// use spacetimedb::{LocalReadOnly, SpacetimeType, ViewContext};

// #[derive(SpacetimeType, Debug, Default)]
// pub struct CriticalHitChance {
//     pub owner: Owner,
//     pub value: f32,
// }

// /// Finds the critical hit chance stat for sender's active character.
// #[spacetimedb::view(name = critical_chance_view, public)]
// pub fn critical_chance_view(ctx: &ViewContext) -> Option<CriticalHitChance> {
//     let Some(active_character) = ctx.db.active_character_tbl().identity().find(ctx.sender) else {
//         return None;
//     };
//     ctx.db
//         .secondary_stats_tbl()
//         .owner()
//         .find(active_character.owner)
//         .map(|s| MovementSpeed {
//             owner: ms.owner,
//             value: s.data.movement_speed,
//         })
// }
