use spacetimedb::{table, ReducerContext, Table};

/// Regenerates the progression table based on a tiered quadratic formula.
///
/// ### The Formula
/// Each level's cost is computed using:
/// $$EXP_{req} = BASE\_COEFFICIENT \cdot Level^2 \cdot (\lfloor\frac{Level-1}{10}\rfloor + 1)^2$$
///
/// `total_exp` is the rolling sum of all previous levels.
///
/// ### Deterministic Output Reference
/// | Level | Tier | Total EXP (Threshold) | Next Level Step |
/// | :--- | :--- | :--- | :--- |
/// | **1** | 1 | 50 | 50 |
/// | **5** | 1 | 2,750 | 1,250 |
/// | **10** | 1 | 19,250 | 5,000 |
/// | **11** | 2 | 43,450 | 24,200 (Tier Jump) |
/// | **20** | 2 | 516,250 | 80,000 |
/// | **21** | 3 | 714,700 | 198,450 (Tier Jump) |
/// | **30** | 3 | 3,479,500 | 405,000 |
/// | **31** | 4 | 4,248,300 | 768,800 (Tier Jump) |
/// | **40** | 4 | 13,627,500 | 1,280,000 |
/// | **41** | 5 | 15,728,750 | 2,101,250 (Tier Jump) |
/// | **50** | 5 | 39,608,750 | 3,125,000 |
///
/// **Note**: This assumes the following constants:
/// const BASE_COEFFICIENT: u32 = 50;
/// const MAX_LEVEL: u32 = 50;
#[table(name = progression_tbl)]
pub struct Progression {
    /// The level associated with the total experience
    #[primary_key]
    pub level: u8,

    /// The total experience points required to reach the level for this ProgressionSystem row
    pub total_exp: u32,
}

impl Progression {
    const BASE_COEFFICIENT: f32 = 200.0;
    const MAX_LEVEL: u8 = 50;

    pub fn new(level: u8, total_exp: u32) -> Self {
        Self { level, total_exp }
    }

    pub fn regenerate(ctx: &ReducerContext) {
        ctx.db.progression_tbl().iter().for_each(|row| {
            ctx.db.progression_tbl().delete(row);
        });

        let mut total_exp: u32 = 0;
        for level in 1..=Self::MAX_LEVEL {
            let tier = ((level - 1) / 10) as i32;

            // Each tier is roughly 2x harder than the previous one
            let tier_multiplier = 2.0f32.powi(tier);
            let level_sq = (level as u32).pow(2) as f32;
            let exp_for_this_level =
                (Self::BASE_COEFFICIENT as f32 * level_sq * tier_multiplier).round() as u32;

            total_exp += exp_for_this_level;

            ctx.db
                .progression_tbl()
                .insert(Progression::new(level, total_exp));
        }
    }
}
