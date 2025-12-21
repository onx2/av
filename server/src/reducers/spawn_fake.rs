use crate::{
    schema::{
        actor, fake_wander_state, movement_data, primary_stats, secondary_stats, transform_data,
        vital_stats, Actor, FakeWanderState, MovementData, PrimaryStats, SecondaryStats,
        TransformData, VitalStats,
    },
    types::{DbVec3, MoveIntent},
};
use shared::utils::encode_cell_id;
use spacetimedb::*;

/// How often the scheduled reducer wakes up to check which fakes need new targets.
const FAKE_WANDER_TICK_HZ: i64 = 2; // 2 Hz = every 500ms

/// Spawn radius around origin for initial placement (meters).
const SPAWN_RADIUS_M: f32 = 100.0;

/// Wander radius around each fake's "home" position (meters).
const DEFAULT_WANDER_RADIUS_M: f32 = 35.0;

/// Random delay range between wander target picks (seconds).
const WANDER_DELAY_MIN_S: i64 = 3;
const WANDER_DELAY_MAX_S: i64 = 10;

/// Default movement speed for fakes (m/s).
const DEFAULT_FAKE_SPEED_MPS: f32 = 3.5;

/// Default capsule dimensions (meters).
const DEFAULT_CAPSULE_RADIUS_M: f32 = 0.35;
const DEFAULT_CAPSULE_HALF_HEIGHT_M: f32 = 0.90;

/// Scheduled driver for fake wandering.
pub fn init(ctx: &ReducerContext) {
    let interval_micros = 1_000_000i64 / FAKE_WANDER_TICK_HZ;
    let tick_interval = TimeDuration::from_micros(interval_micros);

    // Single-row scheduled job.
    ctx.db.fake_wander_tick_timer().scheduled_id().delete(1);
    ctx.db.fake_wander_tick_timer().insert(FakeWanderTickTimer {
        scheduled_id: 1,
        scheduled_at: spacetimedb::ScheduleAt::Interval(tick_interval),
    });
}

/// Scheduled driver for fake wandering. Runs periodically and updates fakes whose timer has elapsed.
#[table(name = fake_wander_tick_timer, scheduled(fake_wander_tick_reducer))]
pub struct FakeWanderTickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,

    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: spacetimedb::ScheduleAt,
}

/// CLI-callable reducer: spawns `count` non-player actors around origin and starts wandering.
///
/// Example:
/// - `spacetime call <db> spawn_fake 25`
#[spacetimedb::reducer]
pub fn spawn_fake(ctx: &ReducerContext, count: u32) -> Result<(), String> {
    if count == 0 {
        return Ok(());
    }

    // Use timestamp-derived deterministic entropy so this remains deterministic for the module.
    let mut rng = SimpleRng::from_timestamp(ctx.timestamp);

    for _ in 0..count {
        // Random initial location around origin, planar (XZ). Keep y=0 and let KCC settle.
        let (sx, sz) = rng.random_point_in_disc(SPAWN_RADIUS_M);
        let spawn_translation = DbVec3::new(sx, 0.0, sz);

        // Create baseline stat rows.
        let primary = ctx.db.primary_stats().insert(PrimaryStats {
            id: 0,
            strength: 5,
            dexterity: 5,
            fortitude: 5,
            intelligence: 5,
            piety: 5,
        });

        let secondary = ctx.db.secondary_stats().insert(SecondaryStats {
            id: 0,
            movement_speed: DEFAULT_FAKE_SPEED_MPS,
            max_health: 100,
            max_mana: 50,
            max_stamina: 100,
        });

        let vital = ctx.db.vital_stats().insert(VitalStats {
            id: 0,
            health: 100,
            mana: 50,
            stamina: 100,
        });

        let transform = ctx.db.transform_data().insert(TransformData {
            id: 0,
            translation: spawn_translation,
            yaw: 0.0,
        });

        let movement = ctx.db.movement_data().insert(MovementData {
            id: 0,
            should_move: false,
            move_intent: MoveIntent::None,
            grounded: false,
            grounded_grace_steps: 0,
        });

        let actor = ctx.db.actor().insert(Actor {
            id: 0,
            primary_stats_id: primary.id,
            secondary_stats_id: secondary.id,
            vital_stats_id: vital.id,
            transform_data_id: transform.id,
            movement_data_id: movement.id,
            identity: None,
            is_player: false,
            cell_id: encode_cell_id(spawn_translation.x, spawn_translation.z),
            capsule_radius: DEFAULT_CAPSULE_RADIUS_M,
            capsule_half_height: DEFAULT_CAPSULE_HALF_HEIGHT_M,
        });

        // Register fake wandering state.
        let delay_s = rng.gen_range_i64_inclusive(WANDER_DELAY_MIN_S, WANDER_DELAY_MAX_S);
        let next_at = ctx
            .timestamp
            .checked_add(TimeDuration::from_micros(delay_s.saturating_mul(1_000_000)))
            .ok_or_else(|| "timestamp overflow computing next wander time".to_string())?;

        ctx.db.fake_wander_state().insert(FakeWanderState {
            actor_id: actor.id,
            home_translation: spawn_translation,
            wander_radius_m: DEFAULT_WANDER_RADIUS_M,
            next_wander_at: next_at,
        });
    }

    Ok(())
}

/// Scheduled reducer that periodically assigns new random MoveIntent targets to fake actors.
#[spacetimedb::reducer]
pub fn fake_wander_tick_reducer(
    ctx: &ReducerContext,
    _timer: FakeWanderTickTimer,
) -> Result<(), String> {
    // Only the server (module identity) may invoke scheduled reducers.
    if ctx.sender != ctx.identity() {
        return Err("`fake_wander_tick_reducer` may not be invoked by clients.".into());
    }

    let mut rng = SimpleRng::from_timestamp(ctx.timestamp);

    // Iterate all fakes whose timer has elapsed and pick a new target within radius of home.
    // NOTE: If this grows large, add a btree index on `next_wander_at` and filter by it.
    for mut state in ctx.db.fake_wander_state().iter() {
        if state.next_wander_at > ctx.timestamp {
            continue;
        }

        let Some(a) = ctx.db.actor().id().find(state.actor_id) else {
            // Orphan state; clean it up.
            ctx.db.fake_wander_state().actor_id().delete(state.actor_id);
            continue;
        };

        if a.is_player {
            // Shouldn't happen, but avoid touching player-controlled actors.
            continue;
        }

        let Some(t) = ctx.db.transform_data().id().find(a.transform_data_id) else {
            continue;
        };

        let Some(mut m) = ctx.db.movement_data().id().find(a.movement_data_id) else {
            continue;
        };

        let (dx, dz) = rng.random_point_in_disc(state.wander_radius_m);

        // Keep y from current transform (KCC will adjust as it moves).
        let target = DbVec3::new(
            state.home_translation.x + dx,
            t.translation.y,
            state.home_translation.z + dz,
        );

        // Set a new target.
        m.move_intent = MoveIntent::Point(target);
        m.should_move = true;
        ctx.db.movement_data().id().update(m);

        // Schedule the next change.
        let delay_s = rng.gen_range_i64_inclusive(WANDER_DELAY_MIN_S, WANDER_DELAY_MAX_S);
        state.next_wander_at = ctx
            .timestamp
            .checked_add(TimeDuration::from_micros(delay_s.saturating_mul(1_000_000)))
            .ok_or_else(|| "timestamp overflow computing next wander time".to_string())?;
        ctx.db.fake_wander_state().actor_id().update(state);
    }

    Ok(())
}

/// Minimal deterministic PRNG suitable for dev tooling (not cryptographic).
///
/// Uses xorshift64* with timestamp-derived seed.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn from_timestamp(ts: Timestamp) -> Self {
        // Timestamp supports debug formatting and deterministic value; we avoid parsing strings.
        // SpacetimeDB Timestamp exposes `.micros_since_epoch()` in some versions; if not, fall back
        // to hashing the debug output. We keep both paths to reduce fragility.
        let seed = timestamp_to_u64(ts);
        let mixed = splitmix64(seed ^ 0xA5A5_A5A5_5A5A_5A5A);
        Self { state: mixed }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_f32_01(&mut self) -> f32 {
        // Convert top 24 bits to f32 in [0, 1).
        let v = (self.next_u64() >> 40) as u32; // 24 bits
        (v as f32) / (1u32 << 24) as f32
    }

    fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32_01()
    }

    fn gen_range_i64_inclusive(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let span = (max - min + 1) as u64;
        let v = self.next_u64() % span;
        min + v as i64
    }

    /// Returns a random point uniformly inside a disc of radius `r` in XZ.
    fn random_point_in_disc(&mut self, r: f32) -> (f32, f32) {
        // Polar method: radius = sqrt(u) * R, angle = 2πv
        let u = self.next_f32_01();
        let v = self.next_f32_01();

        let radius = u.sqrt() * r;
        let theta = v * core::f32::consts::TAU;

        (radius * theta.cos(), radius * theta.sin())
    }
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn timestamp_to_u64(ts: Timestamp) -> u64 {
    // Try a numeric-based approach if available in your SpacetimeDB version.
    // If this doesn't compile, replace with whatever accessor your Timestamp provides.
    #[allow(unused_mut)]
    let mut seed: u64;

    // Common accessor in some versions:
    // - micros since epoch (i64/u64)
    // If not available, the fallback below will be used by commenting this out.
    //
    // seed = ts.micros_since_epoch() as u64;

    // Fallback: hash the Debug string (deterministic for a given Timestamp).
    let s = format!("{ts:?}");
    let mut h: u64 = 14695981039346656037; // FNV-1a offset basis
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    seed = h;

    seed
}
