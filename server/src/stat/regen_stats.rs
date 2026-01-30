use shared::Owner;
use spacetimedb::{reducer, table, ReducerContext, ScheduleAt, SpacetimeType, Table, ViewContext};
use std::{collections::HashMap, time::Duration};

use crate::{health_tbl, mana_tbl};

#[table(name=regen_stats_tbl)]
pub struct RegenStats {
    #[primary_key]
    pub owner: Owner,

    pub data: RegenStatsData,
}

impl RegenStats {
    pub fn find(ctx: &ViewContext, owner: Owner) -> Option<Self> {
        ctx.db.regen_stats_tbl().owner().find(owner)
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: RegenStatsData) {
        ctx.db.regen_stats_tbl().insert(Self { owner, data });
    }
}

/// Regen bonus multipliers (normalized):
/// - 0.0 => +0% (1.0x)
/// - 1.0 => +100% (2.0x)
/// - 5.037 => +503.7% (6.037x)
#[derive(SpacetimeType, Debug, PartialEq, Clone, Copy)]
pub struct RegenStatsData {
    pub health_regen_bonus: f32,
    pub mana_regen_bonus: f32,
}

impl RegenStatsData {
    pub fn compute_regen_rate(base_rate: f32, bonus: f32) -> f32 {
        // If you ever want debuffs to reduce regen, remove `.max(0.0)` and clamp lower elsewhere.
        base_rate * (1.0 + bonus.max(0.0))
    }
}

#[spacetimedb::table(
    name = regen_tick_timer,
    scheduled(regen_reducer)
)]
pub struct RegenTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

/// Regen tick rate is once per second, amount changes per player/monster
const DT_MILLIS: u64 = 1000;
pub fn init_health_and_mana_regen(ctx: &ReducerContext) {
    ctx.db.regen_tick_timer().scheduled_id().delete(&1);
    ctx.db.regen_tick_timer().insert(RegenTimer {
        scheduled_id: 1,
        scheduled_at: Duration::from_millis(DT_MILLIS).into(),
    });
}

#[reducer]
fn regen_reducer(ctx: &ReducerContext, _timer: RegenTimer) -> Result<(), String> {
    /// Base regen rate: fraction-of-max per second (1%).
    const BASE_REGEN_RATE: f32 = 0.01;
    let dt_secs: f32 = DT_MILLIS as f32 / 1000.0;

    // Computes the delta change, though this is essentially moot since we regen at 1second right now
    let compute_delta = |max: u16, rate: f32| ((max as f32) * rate * dt_secs).min(10.0) as u16;

    let mut regen_cache: HashMap<Owner, RegenStatsData> = HashMap::new();
    for health_row in ctx.db.health_tbl().is_full().filter(false) {
        let Some(row) = ctx.db.regen_stats_tbl().owner().find(&health_row.owner) else {
            continue;
        };
        regen_cache.insert(health_row.owner, row.data);

        let max = health_row.data.max;
        let rate = RegenStatsData::compute_regen_rate(BASE_REGEN_RATE, row.data.health_regen_bonus);
        health_row.add(ctx, compute_delta(max, rate));
    }
    for mana_row in ctx.db.mana_tbl().is_full().filter(false) {
        // Try to get regen info from in-memory cache instead of a DB index seek
        let stats: RegenStatsData = if let Some(v) = regen_cache.get(&mana_row.owner) {
            *v
        } else if let Some(row) = ctx.db.regen_stats_tbl().owner().find(&mana_row.owner) {
            row.data
        } else {
            continue;
        };

        let max = mana_row.data.max;
        let rate = RegenStatsData::compute_regen_rate(BASE_REGEN_RATE, stats.health_regen_bonus);
        mana_row.add(ctx, compute_delta(max, rate));
    }
    Ok(())
}
