use crate::{schema::*, types::*};
use spacetimedb::{ReducerContext, Table};

/// Debug/stress-test reducer.
///
/// Spawns a number of fake "remote" actors (implemented as `ActorKind::Monster`)
/// in a random-ish ring around the caller's current actor and gives them an
/// initial wander `MoveIntent::Point`.
///
/// Notes / design choices:
/// - This reducer is intended for local testing and is not permissioned beyond
///   requiring the caller to be in-world.
/// - We avoid external RNG crates. Instead, we derive deterministic-ish pseudo-random
///   values from `(ctx.timestamp, ctx.sender, loop_index)`. This is good enough for
///   a stress test and keeps the module dependency-free.
/// - Movement is driven by the existing `tick` reducer, which already processes
///   `MoveIntent::Point`.
///
/// Limitations:
/// - There is currently no table to track "fake remote players". These are simply
///   additional actors in the `actor` table.
/// - There is no periodic re-targeting in this reducer. For continuous wandering,
///   you can call this reducer repeatedly, or extend the server with a scheduled
///   wander update step that refreshes `move_intent` for `ActorKind::Monster(_)` actors.
#[spacetimedb::reducer]
pub fn spawn_fake_remotes(ctx: &ReducerContext, count: u32) -> Result<(), String> {
    // Require the caller to have a live actor.
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return Err("Player not found".into());
    };
    let Some(source_actor_id) = player.actor_id else {
        return Err("Player is not in-world (no actor)".into());
    };
    let Some(source_actor) = ctx.db.actor().id().find(source_actor_id) else {
        return Err("Source actor not found".into());
    };

    // Clamp count to something reasonable to avoid accidental foot-guns.
    let count = count.clamp(0, 500);

    // Spawn around the local player in an annulus.
    // (Far enough to avoid immediate overlaps, but near enough to see.)
    let spawn_radius_min: f32 = 4.0;
    let spawn_radius_max: f32 = 14.0;

    // Give them a wander target offset so they start moving.
    let wander_radius_min: f32 = 2.0;
    let wander_radius_max: f32 = 10.0;

    // Basic movement/collider settings for stress actors.
    let capsule_radius: f32 = source_actor.capsule_radius;
    let capsule_half_height: f32 = source_actor.capsule_half_height;
    let movement_speed: f32 = (source_actor.movement_speed * 0.9).max(1.0);

    for i in 0..count {
        // Pseudo-random 0..1 values derived from timestamp/sender/index.
        let r0 = prand01(ctx, i, 0);
        let r1 = prand01(ctx, i, 1);
        let r2 = prand01(ctx, i, 2);
        let r3 = prand01(ctx, i, 3);

        // Uniform-ish angle and radius (radius biased slightly outward by sqrt).
        let theta = r0 * core::f32::consts::TAU;
        let radius = lerp(spawn_radius_min, spawn_radius_max, r1.sqrt());

        let spawn_x = source_actor.translation.x + theta.cos() * radius;
        let spawn_z = source_actor.translation.z + theta.sin() * radius;

        // Keep Y same as the local player; KCC will resolve onto ground via snap/bias.
        let spawn_y = source_actor.translation.y;

        // Choose an initial wander target relative to the spawn point.
        let w_theta = r2 * core::f32::consts::TAU;
        let w_radius = lerp(wander_radius_min, wander_radius_max, r3.sqrt());

        let target_x = spawn_x + w_theta.cos() * w_radius;
        let target_z = spawn_z + w_theta.sin() * w_radius;

        // Insert the actor.
        ctx.db.actor().insert(Actor {
            id: 0,
            kind: ActorKind::Monster(i), // "fake remote"
            translation: DbVec3::new(spawn_x, spawn_y, spawn_z),
            yaw: theta,
            capsule_radius,
            capsule_half_height,
            movement_speed,
            move_intent: MoveIntent::Point(DbVec3::new(target_x, spawn_y, target_z)),
            grounded: false,
            grounded_grace_steps: 0,
        });
    }

    Ok(())
}

/// A tiny deterministic pseudo-random generator yielding a float in [0, 1).
///
/// This is *not* crypto-secure; it's intentionally simple and dependency-free.
/// It is good enough for stress-test scattering.
///
/// Implementation: a lightweight mix into a 32-bit value, then map to 0..1.
fn prand01(ctx: &ReducerContext, i: u32, salt: u32) -> f32 {
    // Mix timestamp and sender identity into a u64-ish domain, then hash down.
    // We don't assume any specific layout for Identity; we rely on its Debug string.
    // This is not ideal for performance, but it's fine for debug tooling and avoids
    // requiring Identity->integer conversions.
    let sender_str = format!("{:?}", ctx.sender);
    let mut h: u32 = 2166136261u32; // FNV-1a offset basis

    // Include timestamp (best-effort; Timestamp is Debuggable).
    let ts_str = format!("{:?}", ctx.timestamp);
    for b in ts_str.as_bytes().iter().chain(sender_str.as_bytes().iter()) {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619u32);
    }

    // Incorporate indices and salt.
    h ^= i.wrapping_mul(0x9E37_79B9);
    h = h.rotate_left(13).wrapping_mul(0x85EB_CA6B);
    h ^= salt.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h = h.wrapping_mul(0x27D4_EB2D);
    h ^= h >> 15;

    // Map to [0,1). Use 24 bits of mantissa precision.
    let mantissa = (h >> 8) as u32; // 24 high-ish bits
    (mantissa as f32) / ((1u32 << 24) as f32)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
