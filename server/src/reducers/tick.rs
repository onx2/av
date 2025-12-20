use std::collections::HashSet;

use crate::schema::{actor_in_aoi, Actor, ActorInAoi};
use crate::types::{ActorKind, MoveIntent};
use crate::{
    schema::{actor, kcc_settings},
    tick_timer,
    utils::{get_fixed_delta_time, get_variable_delta_time, has_support_within},
    world::world_query_world,
    TickTimer,
};
use shared::utils::get_aoi_block;
use shared::{
    rapier_world::rapier3d::{
        control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
        prelude::*,
    },
    utils::{encode_cell_id, yaw_from_xz, UtilMath},
};
use spacetimedb::{ReducerContext, Table};

/// Safety cap to avoid spending unbounded time catching up after long stalls.
const MAX_STEPS_PER_TICK: u32 = 5;

#[spacetimedb::reducer]
pub fn tick(ctx: &ReducerContext, mut timer: TickTimer) -> Result<(), String> {
    // Only the server (module identity) may invoke the scheduled reducer.
    if ctx.sender != ctx.identity() {
        return Err("`tick` may not be invoked by clients.".into());
    }

    let Some(kcc) = ctx.db.kcc_settings().id().find(1) else {
        return Err("Missing kcc_settings row (expected id = 1)".into());
    };
    // Fixed timestep.
    let fixed_dt: f32 = get_fixed_delta_time(timer.scheduled_at);
    // Real time elapsed since last tick; used to advance the accumulator.
    let real_dt: f32 = get_variable_delta_time(ctx.timestamp, timer.last_tick).unwrap_or(fixed_dt);
    // Accumulate real time; drain it in fixed-size steps.
    timer.time_accumulator += real_dt;

    // Cache the immutable world query state once per reducer invocation.
    let world = world_query_world(ctx);

    let controller = KinematicCharacterController {
        offset: CharacterLength::Absolute(kcc.offset),
        max_slope_climb_angle: kcc.max_slope_climb_deg.to_radians(),
        min_slope_slide_angle: kcc.min_slope_slide_deg.to_radians(),
        snap_to_ground: None,
        autostep: Some(CharacterAutostep {
            max_height: CharacterLength::Absolute(kcc.autostep_max_height),
            min_width: CharacterLength::Absolute(kcc.autostep_min_width),
            include_dynamic_bodies: false,
        }),
        slide: kcc.slide,
        normal_nudge_factor: kcc.normal_nudge_factor,
        ..KinematicCharacterController::default()
    };

    let mut steps_ran: u32 = 0;
    while timer.time_accumulator >= fixed_dt {
        // Borrowed query pipeline view for this step (Rapier 0.31).
        let query_pipeline = world.query_pipeline(QueryFilter::default());

        // Process all actors.
        for mut actor in ctx.db.actor().iter() {
            // Determine desired planar movement for this fixed step.
            // For now, handle only MoveIntent::Point; other intents result in no planar motion.
            let (target_x, target_z, has_point_intent) = match &actor.move_intent {
                MoveIntent::Point(p) => (p.x, p.z, true),
                _ => (actor.translation.x, actor.translation.z, false),
            };

            // Compute intended planar direction toward the target.
            let dx = target_x - actor.translation.x;
            let dz = target_z - actor.translation.z;

            // Avoid sqrt unless we actually need a direction.
            let dist_sq = dx.sq() + dz.sq();

            // Planar step length.
            let max_step = if has_point_intent {
                actor.movement_speed.max(0.0) * fixed_dt
            } else {
                0.0
            };

            let planar = if dist_sq > 1.0e-12 && max_step > 0.0 {
                let dist = dist_sq.sqrt();
                let inv_dist = 1.0 / dist;
                let step = max_step.min(dist);
                vector![dx * inv_dist * step, 0.0, dz * inv_dist * step]
            } else {
                vector![0.0, 0.0, 0.0]
            };

            // Update yaw based on *intent* direction (not post-collision motion).
            // This looks more natural when sliding along walls.
            if let Some(yaw) = yaw_from_xz(planar.x, planar.z) {
                actor.yaw = yaw;
            }

            // Apply downward bias always; apply fall speed only if we were airborne last step.
            let down_bias = -kcc.grounded_down_bias_mps * fixed_dt;

            // `actor.grounded` is persisted and represents the grounded state from the previous fixed step.
            // This gives us the desired 1-tick lag without any global in-memory cache.
            let gravity = f32::from(!actor.grounded) * (-kcc.fall_speed_mps * fixed_dt);

            let corrected = controller.move_shape(
                fixed_dt,
                &query_pipeline,
                &Capsule::new_y(actor.capsule_half_height, actor.capsule_radius),
                &Isometry::translation(
                    actor.translation.x,
                    actor.translation.y,
                    actor.translation.z,
                ),
                vector![planar.x, down_bias + gravity, planar.z],
                |_| {},
            );

            // Apply corrected movement.
            actor.translation.x += corrected.translation.x;
            actor.translation.y += corrected.translation.y;
            actor.translation.z += corrected.translation.z;

            let new_cell_id = encode_cell_id(actor.translation.x, actor.translation.z);
            if new_cell_id != actor.cell_id {
                actor.cell_id = new_cell_id;
            }

            match actor.kind {
                // ActorKind::Player(identity) => {
                //     let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);

                //     // 1. Target: The actors that ARE in the 9 neighbor cells
                //     let target_actors: Vec<Actor> = aoi_block
                //         .into_iter()
                //         .flat_map(|cell_id| ctx.db.actor().cell_id().filter(cell_id))
                //         .collect();

                //     // 2. Current: The actors this player THINKS are in their AOI
                //     let current_aoi_rows: Vec<ActorInAoi> =
                //         ctx.db.actor_in_aoi().identity().filter(identity).collect();

                //     // Maps for fast lookups
                //     let target_ids: std::collections::HashSet<u64> =
                //         target_actors.iter().map(|a| a.id).collect();

                //     // 3. DELETE phase: Actors that moved too far away
                //     for row in current_aoi_rows.iter() {
                //         if !target_ids.contains(&row.actor_id) {
                //             ctx.db.actor_in_aoi().delete(ActorInAoi { ..*row });
                //         }
                //     }

                //     // 4. INSERT / UPDATE phase
                //     for actor_to_sync in target_actors {
                //         // Find if we already have a record for this specific actor in this player's AOI
                //         let existing_record = current_aoi_rows
                //             .iter()
                //             .find(|r| r.actor_id == actor_to_sync.id);

                //         match existing_record {
                //             Some(existing) => {
                //                 ctx.db.actor_in_aoi().id().update(ActorInAoi {
                //                     id: existing.id,
                //                     identity,
                //                     actor_id: actor_to_sync.id,
                //                     translation: actor_to_sync.translation,
                //                     yaw: actor_to_sync.yaw,
                //                     kind: existing.kind,
                //                     capsule_radius: existing.capsule_radius,
                //                     capsule_half_height: existing.capsule_half_height,
                //                 });
                //             }
                //             None => {
                //                 // INSERT: Brand new actor entering the player's view
                //                 ctx.db.actor_in_aoi().insert(ActorInAoi {
                //                     id: 0,
                //                     identity,
                //                     actor_id: actor_to_sync.id,
                //                     translation: actor_to_sync.translation,
                //                     kind: actor_to_sync.kind,
                //                     yaw: actor_to_sync.yaw,
                //                     capsule_radius: actor_to_sync.capsule_radius,
                //                     capsule_half_height: actor_to_sync.capsule_half_height,
                //                 });
                //             }
                //         }
                //     }
                // }
                ActorKind::Player(identity) => {
                    let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);
                    let target_actors: Vec<Actor> = aoi_block
                        .into_iter()
                        .flat_map(|cell_id| ctx.db.actor().cell_id().filter(cell_id))
                        .collect();

                    let target_ids: HashSet<u64> = target_actors.iter().map(|a| a.id).collect();

                    // Delete outdated
                    for row in ctx.db.actor_in_aoi().identity().filter(identity) {
                        if !target_ids.contains(&row.actor_id) {
                            ctx.db.actor_in_aoi().delete(row);
                        }
                    }

                    // Insert/update new
                    let idx = ctx.db.actor_in_aoi().identity_actor();
                    for actor_to_sync in target_actors {
                        if let Some(existing) = idx.filter((identity, actor_to_sync.id)).next() {
                            ctx.db.actor_in_aoi().id().update(ActorInAoi {
                                id: existing.id,
                                identity,
                                actor_id: actor_to_sync.id,
                                translation: actor_to_sync.translation,
                                yaw: actor_to_sync.yaw,
                                kind: existing.kind, // or actor_to_sync.kind if updatable
                                capsule_radius: existing.capsule_radius,
                                capsule_half_height: existing.capsule_half_height,
                            });
                        } else {
                            ctx.db.actor_in_aoi().insert(ActorInAoi {
                                id: 0,
                                identity,
                                actor_id: actor_to_sync.id,
                                translation: actor_to_sync.translation,
                                kind: actor_to_sync.kind,
                                yaw: actor_to_sync.yaw,
                                capsule_radius: actor_to_sync.capsule_radius,
                                capsule_half_height: actor_to_sync.capsule_half_height,
                            });
                        }
                    }
                }
                _ => {}
            }
            // Persist grounded for the next fixed step.
            if corrected.grounded {
                actor.grounded = true;
                actor.grounded_grace_steps = 8;
            } else if actor.grounded_grace_steps > 0 {
                let supported = has_support_within(
                    &query_pipeline,
                    &actor,
                    kcc.hard_airborne_probe_distance,
                    kcc.max_slope_climb_deg.to_radians().cos(),
                );

                if supported {
                    actor.grounded_grace_steps -= 1;
                } else {
                    actor.grounded_grace_steps = 0;
                    actor.grounded = false;
                }
            } else {
                actor.grounded = false;
            }

            // Clear MoveIntent::Point when within the acceptance radius (planar).
            // TODO: Acceptance radius should be computed differently
            if has_point_intent && dist_sq <= kcc.point_acceptance_radius_sq {
                actor.move_intent = MoveIntent::None;
            }

            ctx.db.actor().id().update(actor);
        }

        // Consume fixed time step.
        timer.time_accumulator -= fixed_dt;
        steps_ran += 1;

        if steps_ran >= MAX_STEPS_PER_TICK {
            // Prevent runaway catch-up loops.
            timer.time_accumulator = 0.0;
            break;
        }
    }

    // Persist timer state.
    timer.last_tick = ctx.timestamp;
    ctx.db.tick_timer().scheduled_id().update(timer);

    Ok(())
}
