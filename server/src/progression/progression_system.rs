const BASE_COEFFICIENT: u32 = 200;
const MAX_LEVEL: usize = 50;

const fn generate_progression() -> [u32; MAX_LEVEL] {
    let mut table = [0u32; MAX_LEVEL];
    let mut total_accumulated_exp: u32 = 0;
    let mut i = 0;

    while i < MAX_LEVEL {
        // Assign current total (Level 1 starts at 0)
        table[i] = total_accumulated_exp;

        let current_level = (i + 1) as u32;

        // Tier shifts every 10 levels: i=0..9 (T0), i=10..19 (T1), etc.
        let tier = (i / 10) as u32;
        let tier_multiplier = 1 << tier; // 2^tier

        let level_sq = current_level * current_level;
        let exp_to_next_level = BASE_COEFFICIENT * level_sq * tier_multiplier;

        total_accumulated_exp += exp_to_next_level;
        i += 1;
    }
    table
}

pub const EXPERIENCE_PER_LEVEL: [u32; MAX_LEVEL] = generate_progression();
