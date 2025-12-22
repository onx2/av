//! Soft-collision + idle stacking ("tuck") + mutual drift.
//!
//! Goal:
//! - Ensure a walking/moving player can never "push" a standing/idle player around the map.
//!
//! Rules (Static Anchoring):
//! - Immovable IDLE: If an actor is IDLE, they are a static obstacle (infinite mass). Their position
//!   must never be modified by a MOVING actor.
//! - Steering vs. resolution:
//!   - While MOVING, an actor uses separation steering to avoid IDLE actors (ONLY the mover is displaced).
//!   - IDLE actors remain at V = 0 (no reaction) during this interaction.
//! - The tuck-in event (MOVING -> IDLE):
//!   - The only time an IDLE actor may be moved due to another actor is at the moment it transitions
//!     to IDLE while overlapping hard radii. Then it nudges itself to the nearest clear spot.
//! - Mutual drift (IDLE-IDLE hard overlap):
//!   - If two IDLE actors are overlapping hard radii (spawn/teleport), apply a very slow mutual
//!     repulsion so they eventually settle side-by-side.
//!
//! Execution:
//! - Kinematic position updates only (KCC move_shape), no physics rigid bodies.
//! - Runs at a low fixed rate (scheduled tick), with strict caps for performance.
//!
//! Cell scoping:
//! - For performance, we only consider actors in the same `cell_id`.
//!   This can miss rare cross-cell interactions near boundaries, acceptable by design.

use crate::{
    schema::{actor, transform_data},
    utils::get_fixed_delta_time,
    world::{get_kcc, get_rapier_world},
};

use rapier3d::prelude::{Isometry, QueryFilter};
use spacetimedb::{ReducerContext, ScheduleAt, Table, TimeDuration, Timestamp};

use shared::{
    constants::{
        IDLE_OVERLAP_PUSH_TICK_HZ, OVERLAP_PUSH_MAX_STEP_M, OVERLAP_PUSH_SKIN_M, Y_QUANTIZE_STEP_M,
    },
    utils::{encode_cell_id, UtilMath},
};

use crate::types::MoveIntent;

#[spacetimedb::table(
    name = idle_overlap_push_tick_timer,
    scheduled(idle_overlap_push_tick_reducer)
)]
pub struct IdleOverlapPushTickTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
    pub last_tick: Timestamp,
}

pub fn init(ctx: &ReducerContext) {
    let interval = TimeDuration::from_micros(1_000_000i64 / IDLE_OVERLAP_PUSH_TICK_HZ);

    ctx.db
        .idle_overlap_push_tick_timer()
        .scheduled_id()
        .delete(1);
    ctx.db
        .idle_overlap_push_tick_timer()
        .insert(IdleOverlapPushTickTimer {
            scheduled_id: 1,
            scheduled_at: ScheduleAt::Interval(interval),
            last_tick: ctx.timestamp,
        });
}

// ------------------------------------------------------------
// Tuning constants
// ------------------------------------------------------------

/// Additional personal-space radius beyond the capsule radius (meters).
const SOFT_BUFFER_M: f32 = 0.35;

/// Mutual-drift repulsion step factor (0..1) per tick (smaller = gentler).
///
/// This is intentionally *very* small so stacked idle actors separate slowly without looking like
/// they're being "pushed" by gameplay movement.
const MUTUAL_IDLE_REPULSION_RELAX: f32 = 0.05;

/// Steering strength inside the soft buffer (0..1). Higher = stronger avoidance.
const SOFT_STEER_STRENGTH: f32 = 0.65;

/// Max number of actors processed per cell in this tick (safety/perf cap).
const MAX_ACTORS_PER_CELL: usize = 32;

/// Max number of neighbor checks per actor (safety/perf cap).
const MAX_NEIGHBORS_PER_ACTOR: usize = 64;

/// Number of samples around the actor for tuck search.
const TUCK_DIR_SAMPLES: usize = 24;

/// Number of radial rings tested for tuck search.
const TUCK_RINGS: usize = 6;

/// Step size (meters) between rings for tuck search.
const TUCK_RING_STEP_M: f32 = 0.25;

// ------------------------------------------------------------
// Helpers
// ------------------------------------------------------------

#[inline]
fn clamp_planar(dx: f32, dz: f32, max_len: f32) -> (f32, f32) {
    let len_sq = dx.sq() + dz.sq();
    if len_sq <= max_len.sq() || len_sq <= 1.0e-12 {
        return (dx, dz);
    }
    let len = len_sq.sqrt();
    let s = max_len / len;
    (dx * s, dz * s)
}

#[inline]
fn dist_sq_xz(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    (ax - bx).sq() + (az - bz).sq()
}

/// Returns Some((penetration, nx, nz)) for hard overlap (min_dist = rA + rB + skin).
#[inline]
fn hard_overlap_penetration_normal(
    ax: f32,
    az: f32,
    ar: f32,
    bx: f32,
    bz: f32,
    br: f32,
) -> Option<(f32, f32, f32)> {
    let dx = ax - bx;
    let dz = az - bz;
    let dist_sq = dx.sq() + dz.sq();
    let min_dist = ar + br + OVERLAP_PUSH_SKIN_M;
    let min_dist_sq = min_dist.sq();

    if dist_sq >= min_dist_sq {
        return None;
    }
    if dist_sq <= 1.0e-12 {
        return Some((min_dist, 1.0, 0.0));
    }

    let dist = dist_sq.sqrt();
    let inv = 1.0 / dist;
    let pen = min_dist - dist;
    Some((pen, dx * inv, dz * inv))
}

/// Returns signed "soft intrusion" amount (meters) into the soft buffer.
/// >0 means inside soft zone, <=0 means outside.
#[inline]
fn soft_intrusion(
    moving_x: f32,
    moving_z: f32,
    moving_soft_r: f32,
    stationary_x: f32,
    stationary_z: f32,
    stationary_soft_r: f32,
) -> f32 {
    let dx = moving_x - stationary_x;
    let dz = moving_z - stationary_z;
    let dist_sq = dx.sq() + dz.sq();
    let min_dist = moving_soft_r + stationary_soft_r;
    if dist_sq <= 1.0e-12 {
        return min_dist;
    }
    let dist = dist_sq.sqrt();
    min_dist - dist
}

/// Checks if position (x,z) is hard-clear of all neighbors' hard radii.
fn is_hard_clear(
    x: f32,
    z: f32,
    self_radius: f32,
    neighbors: &[(f32, f32, f32)], // (ox, oz, orad)
) -> bool {
    for (ox, oz, orad) in neighbors.iter().copied() {
        let min_dist = self_radius + orad + OVERLAP_PUSH_SKIN_M;
        if dist_sq_xz(x, z, ox, oz) < min_dist.sq() {
            return false;
        }
    }
    true
}

/// Finds a nearby "tuck" position that is hard-clear, preferring minimal displacement.
fn find_tuck_position(
    x0: f32,
    z0: f32,
    self_radius: f32,
    neighbors: &[(f32, f32, f32)],
) -> Option<(f32, f32)> {
    if is_hard_clear(x0, z0, self_radius, neighbors) {
        return Some((x0, z0));
    }

    // Precomputed directions around circle (cos/sin samples).
    // We compute on the fly using f32 trig; TUCK_RINGS and samples are small, tick is low-rate.
    let mut best: Option<(f32, f32, f32)> = None; // (x, z, dist_sq)

    for ring in 1..=TUCK_RINGS {
        let r = ring as f32 * TUCK_RING_STEP_M;
        for k in 0..TUCK_DIR_SAMPLES {
            let a = (k as f32) * (std::f32::consts::TAU / TUCK_DIR_SAMPLES as f32);
            let cx = x0 + a.cos() * r;
            let cz = z0 + a.sin() * r;

            if is_hard_clear(cx, cz, self_radius, neighbors) {
                let d2 = dist_sq_xz(cx, cz, x0, z0);
                match best {
                    None => best = Some((cx, cz, d2)),
                    Some((_, _, best_d2)) if d2 < best_d2 => best = Some((cx, cz, d2)),
                    _ => {}
                }
            }
        }

        // Early exit: if we found something on this ring, it should be near-minimal.
        if best.is_some() {
            break;
        }
    }

    best.map(|(x, z, _)| (x, z))
}

#[inline]
fn is_idle(actor: &crate::schema::Actor) -> Option<u64> {
    match actor.move_intent {
        MoveIntent::Idle(since_us) => Some(since_us),
        _ => None,
    }
}

// ------------------------------------------------------------
// Main scheduled reducer
// ------------------------------------------------------------

#[spacetimedb::reducer]
pub fn idle_overlap_push_tick_reducer(
    ctx: &ReducerContext,
    mut timer: IdleOverlapPushTickTimer,
) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("`idle_overlap_push_tick_reducer` may not be invoked by clients.".into());
    }

    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);

    let world = get_rapier_world(ctx);
    let controller = get_kcc(ctx);
    let query_pipeline = world.query_pipeline(QueryFilter::default());

    // We'll process each cell once per tick, seeded by the idle-player iterator.
    let mut processed_cells: Vec<u32> = Vec::new();

    // Seed cells from idle players.
    for seed in ctx
        .db
        .actor()
        .should_move_and_is_player()
        .filter((false, true))
    {
        // Only actors that are explicitly Idle participate in this system.
        let Some(_idle_since) = is_idle(&seed) else {
            continue;
        };

        let cell_id = seed.cell_id;
        if processed_cells.iter().any(|c| *c == cell_id) {
            continue;
        }
        processed_cells.push(cell_id);

        // Gather actors in this cell (players only) up to cap.
        // NOTE: IDLE actors are treated as immovable. MOVING actors are only steered; they never push IDLE actors.
        let mut actors: Vec<crate::schema::Actor> = Vec::new();
        actors.reserve(MAX_ACTORS_PER_CELL);

        for a in ctx.db.actor().cell_id().filter(cell_id) {
            if actors.len() >= MAX_ACTORS_PER_CELL {
                break;
            }
            // Include NPCs as obstacles too (they will never be moved by this reducer).
            actors.push(a);
        }

        if actors.len() < 2 {
            continue;
        }

        // Cache transform positions for all gathered actors (avoid re-reading many times).
        // Store: (actor_id, transform_id, x, y_m, z, hard_r, soft_r, half_h, idle_since_opt, should_move)
        let mut cache: Vec<(u64, u32, f32, f32, f32, f32, f32, f32, Option<u64>, bool)> =
            Vec::new();
        cache.reserve(actors.len());

        for a in actors.iter() {
            let Some(t) = ctx.db.transform_data().id().find(a.transform_data_id) else {
                continue;
            };
            let x = t.translation.x;
            let y_m = t.translation.y as f32 * Y_QUANTIZE_STEP_M;
            let z = t.translation.z;

            let hard_r = a.capsule_radius;
            let soft_r = hard_r + SOFT_BUFFER_M;

            let idle_since = is_idle(a);
            cache.push((
                a.id,
                a.transform_data_id,
                x,
                y_m,
                z,
                hard_r,
                soft_r,
                a.capsule_half_height,
                idle_since,
                a.should_move,
            ));
        }

        if cache.len() < 2 {
            continue;
        }

        // ------------------------------------------------------------
        // Phase A: Separation Steering (MOVING -> IDLE)
        //
        // NOTE:
        // This reducer MUST NOT move IDLE actors due to MOVING actors.
        //
        // Therefore, steering is applied ONLY to MOVING actors to avoid IDLE obstacles.
        // IDLE actors remain static (infinite mass).
        // ------------------------------------------------------------
        for i in 0..cache.len() {
            let (aid, tid, x, y_m, z, hard_r, soft_r, half_h, _idle_since, should_move) = cache[i];

            // Only apply steering to moving actors.
            if !should_move {
                continue;
            }

            let mut steer_x = 0.0f32;
            let mut steer_z = 0.0f32;

            for j in 0..cache.len() {
                if i == j {
                    continue;
                }

                let (_bid, _btid, bx, _by, bz, _br, bsoft_r, _bhh, bidle_since, bshould_move) =
                    cache[j];

                // Stationary obstacles (infinite mass for steering purposes):
                // - Players: only if IDLE (have an idle timestamp) and not moving.
                // - NPCs: any non-moving NPC is treated as a stationary obstacle.
                if bshould_move {
                    continue;
                }

                // If `bidle_since` is None, this is not an idle-player entry. Treat it as:
                // - NPC obstacle (allowed), or
                // - non-idle player (ignored for steering).
                let is_stationary_obstacle = bidle_since.is_some() || !actors[j].is_player;
                if !is_stationary_obstacle {
                    continue;
                }

                let intr = soft_intrusion(x, z, soft_r, bx, bz, bsoft_r);
                if intr <= 0.0 {
                    continue;
                }

                let dx = x - bx;
                let dz = z - bz;
                let dist_sq = dx.sq() + dz.sq();
                let (nx, nz) = if dist_sq <= 1.0e-12 {
                    (1.0, 0.0)
                } else {
                    let inv = 1.0 / dist_sq.sqrt();
                    (dx * inv, dz * inv)
                };

                let amount = intr * SOFT_STEER_STRENGTH;
                steer_x += nx * amount;
                steer_z += nz * amount;
            }

            let (steer_x, steer_z) = clamp_planar(steer_x, steer_z, OVERLAP_PUSH_MAX_STEP_M);
            if steer_x.sq() + steer_z.sq() <= 1.0e-12 {
                continue;
            }

            let corrected = controller.move_shape(
                fixed_dt.max(1.0e-6),
                &query_pipeline,
                &rapier3d::prelude::Capsule::new_y(half_h, hard_r),
                &Isometry::translation(x, y_m, z),
                rapier3d::prelude::vector![steer_x, 0.0, steer_z],
                |_| {},
            );

            let new_x = x + corrected.translation.x;
            let new_y = y_m + corrected.translation.y;
            let new_z = z + corrected.translation.z;

            let Some(mut t) = ctx.db.transform_data().id().find(tid) else {
                continue;
            };
            t.translation.x = new_x;
            t.translation.z = new_z;
            t.translation.y = (new_y / Y_QUANTIZE_STEP_M)
                .round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            ctx.db.transform_data().id().update(t);

            let new_cell = encode_cell_id(new_x, new_z);
            if let Some(mut a) = ctx.db.actor().id().find(aid) {
                if new_cell != a.cell_id {
                    a.cell_id = new_cell;
                    ctx.db.actor().id().update(a);
                }
            }

            cache[i].2 = new_x;
            cache[i].3 = new_y;
            cache[i].4 = new_z;
        }

        // ------------------------------------------------------------
        // Phase B: Tuck logic (Idle Resolution)
        //
        // IMPORTANT:
        // - Tuck should ONLY apply to the actor that just became idle (i.e. just stopped moving),
        //   so that long-idle/stationary players are not moved "out of their spot" by arriving movers.
        //
        // Policy:
        // - Only consider actors that are idle+stationary AND have idled very recently.
        // - Use a small "just idled" window (e.g. 500ms) based on `MoveIntent::Idle(idle_since_us)`.
        // - If they hard-overlap anyone, search nearby for the nearest hard-clear position and move ONLY them.
        // ------------------------------------------------------------
        let now_us: u64 = ctx.timestamp.to_micros_since_unix_epoch().max(0) as u64;
        const JUST_IDLED_WINDOW_US: u64 = 500_000;

        for i in 0..cache.len() {
            let (aid, tid, x, y_m, z, hard_r, _soft_r, half_h, idle_since, should_move) = cache[i];

            // Only consider idle + stationary.
            if should_move {
                continue;
            }

            // Only consider actors that just became idle very recently.
            let Some(idle_since_us) = idle_since else {
                continue;
            };
            if now_us.saturating_sub(idle_since_us) > JUST_IDLED_WINDOW_US {
                continue;
            }

            // Build neighbor list (hard radii) from other players in the cell.
            // Include both idle and moving actors as hard obstacles so we don't tuck into someone else.
            let mut neighbors: Vec<(f32, f32, f32)> = Vec::new();
            neighbors.reserve(MAX_NEIGHBORS_PER_ACTOR.min(cache.len().saturating_sub(1)));

            for j in 0..cache.len() {
                if i == j {
                    continue;
                }
                let (_bid, _btid, bx, _by, bz, br, _bsr, _bhh, _bidle, _bsm) = cache[j];
                neighbors.push((bx, bz, br));
                if neighbors.len() >= MAX_NEIGHBORS_PER_ACTOR {
                    break;
                }
            }

            // If overlapping hard radii, try to tuck.
            if !is_hard_clear(x, z, hard_r, &neighbors) {
                if let Some((tx2, tz2)) = find_tuck_position(x, z, hard_r, &neighbors) {
                    let dx = tx2 - x;
                    let dz = tz2 - z;

                    // Apply tuck via KCC.
                    let (dx, dz) = clamp_planar(dx, dz, OVERLAP_PUSH_MAX_STEP_M);
                    if dx.sq() + dz.sq() > 1.0e-12 {
                        let corrected = controller.move_shape(
                            fixed_dt.max(1.0e-6),
                            &query_pipeline,
                            &rapier3d::prelude::Capsule::new_y(half_h, hard_r),
                            &Isometry::translation(x, y_m, z),
                            rapier3d::prelude::vector![dx, 0.0, dz],
                            |_| {},
                        );

                        let new_x = x + corrected.translation.x;
                        let new_y = y_m + corrected.translation.y;
                        let new_z = z + corrected.translation.z;

                        let Some(mut t) = ctx.db.transform_data().id().find(tid) else {
                            continue;
                        };
                        t.translation.x = new_x;
                        t.translation.z = new_z;
                        t.translation.y = (new_y / Y_QUANTIZE_STEP_M)
                            .round()
                            .clamp(i16::MIN as f32, i16::MAX as f32)
                            as i16;
                        ctx.db.transform_data().id().update(t);

                        let new_cell = encode_cell_id(new_x, new_z);
                        if let Some(mut a) = ctx.db.actor().id().find(aid) {
                            if new_cell != a.cell_id {
                                a.cell_id = new_cell;
                                ctx.db.actor().id().update(a);
                            }
                        }

                        cache[i].2 = new_x;
                        cache[i].3 = new_y;
                        cache[i].4 = new_z;
                    }
                }
            }
        }

        // ------------------------------------------------------------
        // Phase C: Mutual drift (IDLE-IDLE hard overlap) — STRICT ANCHORING
        //
        // If two IDLE stationary actors overlap hard radii (spawn/teleport), apply a *very slow*
        // repulsion, but ONLY move the more recently idle actor. The older-idle actor stays anchored.
        //
        // IMPORTANT:
        // - This must never be triggered by a MOVING actor.
        // - It only applies when BOTH actors are IDLE and stationary.
        // - Smaller `idle_since_us` means "idle for longer" => that actor is anchored.
        // ------------------------------------------------------------
        for i in 0..cache.len() {
            for j in (i + 1)..cache.len() {
                let (aid, atid, ax, ay, az, ar, _asr, ahh, aidle, amove) = cache[i];
                let (bid, btid, bx, by, bz, br, _bsr, bhh, bidle, bmove) = cache[j];

                // Only IDLE + stationary on both sides.
                if amove || bmove {
                    continue;
                }
                let (Some(a_idle_since_us), Some(b_idle_since_us)) = (aidle, bidle) else {
                    continue;
                };

                let Some((pen, nx, nz)) = hard_overlap_penetration_normal(ax, az, ar, bx, bz, br)
                else {
                    continue;
                };

                if pen <= 1.0e-5 {
                    continue;
                }

                // Very slow separation, but applied only to the MORE RECENTLY idle actor.
                // (Higher since_us => more recent)
                let step = (pen * MUTUAL_IDLE_REPULSION_RELAX).min(OVERLAP_PUSH_MAX_STEP_M);

                let (move_a, move_b) = if a_idle_since_us > b_idle_since_us {
                    (true, false)
                } else if b_idle_since_us > a_idle_since_us {
                    (false, true)
                } else {
                    // Tie-break: if timestamps are equal, push by stable id ordering (newer/spawned later tends to have higher id).
                    (aid > bid, bid > aid)
                };

                // Apply via KCC (XZ only).
                // A (only if selected to move)
                if move_a {
                    let push_ax = nx * step;
                    let push_az = nz * step;

                    let corrected = controller.move_shape(
                        fixed_dt.max(1.0e-6),
                        &query_pipeline,
                        &rapier3d::prelude::Capsule::new_y(ahh, ar),
                        &Isometry::translation(ax, ay, az),
                        rapier3d::prelude::vector![push_ax, 0.0, push_az],
                        |_| {},
                    );
                    let new_x = ax + corrected.translation.x;
                    let new_y = ay + corrected.translation.y;
                    let new_z = az + corrected.translation.z;

                    if let Some(mut t) = ctx.db.transform_data().id().find(atid) {
                        t.translation.x = new_x;
                        t.translation.z = new_z;
                        t.translation.y = (new_y / Y_QUANTIZE_STEP_M)
                            .round()
                            .clamp(i16::MIN as f32, i16::MAX as f32)
                            as i16;
                        ctx.db.transform_data().id().update(t);
                    }
                    if let Some(mut a) = ctx.db.actor().id().find(aid) {
                        let new_cell = encode_cell_id(new_x, new_z);
                        if new_cell != a.cell_id {
                            a.cell_id = new_cell;
                            ctx.db.actor().id().update(a);
                        }
                    }

                    cache[i].2 = new_x;
                    cache[i].3 = new_y;
                    cache[i].4 = new_z;
                }

                // B (only if selected to move)
                if move_b {
                    let push_bx = -nx * step;
                    let push_bz = -nz * step;

                    let corrected = controller.move_shape(
                        fixed_dt.max(1.0e-6),
                        &query_pipeline,
                        &rapier3d::prelude::Capsule::new_y(bhh, br),
                        &Isometry::translation(bx, by, bz),
                        rapier3d::prelude::vector![push_bx, 0.0, push_bz],
                        |_| {},
                    );
                    let new_x = bx + corrected.translation.x;
                    let new_y = by + corrected.translation.y;
                    let new_z = bz + corrected.translation.z;

                    if let Some(mut t) = ctx.db.transform_data().id().find(btid) {
                        t.translation.x = new_x;
                        t.translation.z = new_z;
                        t.translation.y = (new_y / Y_QUANTIZE_STEP_M)
                            .round()
                            .clamp(i16::MIN as f32, i16::MAX as f32)
                            as i16;
                        ctx.db.transform_data().id().update(t);
                    }
                    if let Some(mut a) = ctx.db.actor().id().find(bid) {
                        let new_cell = encode_cell_id(new_x, new_z);
                        if new_cell != a.cell_id {
                            a.cell_id = new_cell;
                            ctx.db.actor().id().update(a);
                        }
                    }

                    cache[j].2 = new_x;
                    cache[j].3 = new_y;
                    cache[j].4 = new_z;
                }
            }
        }
    }

    timer.last_tick = ctx.timestamp;
    ctx.db
        .idle_overlap_push_tick_timer()
        .scheduled_id()
        .update(timer);

    Ok(())
}
