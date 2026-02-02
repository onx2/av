pub const MAX_LEVEL: u8 = 50;
pub const TIER_INTERVAL: u8 = 10;

const BASE_COEFFICIENT: usize = 200;
const MAX_LEVEL_USIZE: usize = MAX_LEVEL as usize;
const TIER_INTERVAL_USIZE: usize = TIER_INTERVAL as usize;

const fn generate_progression() -> [u32; MAX_LEVEL_USIZE] {
    let mut table = [0u32; MAX_LEVEL_USIZE];
    let mut total: usize = 0;
    let mut i: usize = 0;

    while i < MAX_LEVEL_USIZE {
        table[i] = total as u32;

        let current_level = i + 1;
        let tier = i / TIER_INTERVAL_USIZE;
        let tier_multiplier = 1usize << tier; // 2^tier

        let exp_to_next = BASE_COEFFICIENT * (current_level * current_level) * tier_multiplier;
        total += exp_to_next;

        i += 1;
    }

    table
}

pub const EXPERIENCE_PER_LEVEL: [u32; MAX_LEVEL_USIZE] = generate_progression();
