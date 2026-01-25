use std::u32;

use shared::owner::Owner;
use spacetimedb::{table, SpacetimeType};

#[table(name = progression_tbl)]
pub struct Progression {
    #[primary_key] // or index on owner
    pub owner: Owner,

    pub data: ProgressionData,
}
#[derive(SpacetimeType, Debug, PartialEq, Eq, Clone, Copy)]
pub struct ProgressionData {
    pub level: u8,
    pub exp: u32,
}

impl ProgressionData {
    // TODO
    // /// Returns the TOTAL experience required to reach the NEXT level.
    // /// This uses a Step-Wise Exponential formula (The "Staircase Golden Ratio").
    // pub fn exp_for_next_level(&self) -> u32 {
    //     // Start with a base of 100 XP for Level 1 -> 2
    //     let base_xp = 100.0;
    //     let growth_rate = 1.1f64;
    //     let step_size = 10.0;

    //     // Subtract 1 so that levels 1-9 are in the same "step" (0)
    //     // Level 10-19 becomes step 1, and so on.
    //     let steps = ((self.level as f64 - 1.0) / step_size).floor();
    //     let exponent = steps * step_size;

    //     (base_xp * growth_rate.powf(exponent)) as u32
    // }

    // pub fn add_exp(&mut self, amount: u32) {
    //     self.exp = self.exp.saturating_add(amount);
    //     self.sync();
    // }

    // /// Use this for death penalties or XP drains
    // pub fn sub_exp(&mut self, amount: u32) {
    //     self.exp = self.exp.saturating_sub(amount);
    //     self.sync();
    // }

    // pub fn sync(&mut self) {
    //     while self.exp >= self.exp_for_next_level() {
    //         self.level = self.level.saturating_add(1);
    //     }

    //     while self.level > 1 && self.exp < self.exp_for_requirement_of(self.level) {
    //         self.level = self.level.saturating_sub(1);
    //     }
    // }

    // pub fn calculate_xp_for_next_level(level: u8) -> u32 {
    //     let base_xp = 100.0;
    //     let growth_rate = 1.1f64;
    //     let step_size = 10.0;
    //     let steps = ((level as f64 - 1.0) / step_size).floor();
    //     let exponent = steps * step_size;
    //     (base_xp * growth_rate.powf(exponent)) as u32
    // }

    // /// XP required to BE this level
    // pub fn exp_for_requirement_of(&self, level: u8) -> u32 {
    //     if level <= 1 {
    //         return 0;
    //     }
    //     // The XP required to BE Level 10 is the same as the requirement that was used to get FROM 9 TO 10.
    //     Self::calculate_xp_for_next_level(level - 1)
    // }
}
