// TODO: I think "computed"/"secondary" stats should just be a view instead. we don't need to predict those
// values but instead client can rely on replicated data.

// use std::ops::AddAssign;

// use shared::owner::Owner;
// use spacetimedb::{table, SpacetimeType};

// /// Ephemeral
// ///
// /// The derived / computed stats for an owner, based on various things like PrimaryStats, equipment, perks, spells, etc...
// #[table(name=secondary_stats_tbl)]
// pub struct SecondaryStats {
//     #[primary_key]
//     pub owner: Owner,

//     pub data: SecondaryStatsData,
// }
// #[derive(SpacetimeType, Debug, PartialEq, Clone, Copy)]
// pub struct SecondaryStatsData {
//     pub movement_speed: ComputedStat<f32>,
//     pub critical_hit_chance: ComputedStat<f32>,
// }
// crate::impl_data_table!(
//     table_handle = secondary_stats_tbl,
//     row = SecondaryStats,
//     data = SecondaryStatsData
// );

// impl Default for SecondaryStatsData {
//     fn default() -> Self {
//         Self {
//             movement_speed: ComputedStat::new(5.0),
//             critical_hit_chance: ComputedStat::new(5.0),
//         }
//     }
// }

// #[derive(SpacetimeType, Debug, PartialEq, Clone, Copy)]
// pub struct ComputedStat<T> {
//     pub base: T,
//     pub current: T,
// }
// impl<T> ComputedStat<T>
// where
//     T: AddAssign + Copy,
// {
//     pub fn new(value: T) -> Self {
//         Self {
//             base: value,
//             current: value,
//         }
//     }

//     pub fn reset(&mut self) {
//         self.current = self.base;
//     }

//     pub fn set_base(&mut self, new_base: T) {
//         self.base = new_base;
//         self.current = new_base;
//     }

//     // pub fn apply_modifier(&mut self, modifier: T) {
//     //     self.current += modifier;
//     // }
// }
