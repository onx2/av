use nalgebra as na;
use std::sync::OnceLock;

use spacetimedb::*;

#[derive(SpacetimeType, Clone, Copy, PartialEq)]
pub struct DbQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

fn yaw_to_db_quat(theta: f32) -> DbQuat {
    let half = 0.5 * theta;
    DbQuat {
        x: 0.0,
        y: half.sin(),
        z: 0.0,
        w: half.cos(),
    }
}

impl From<DbVec3> for na::Vector3<f32> {
    fn from(v: DbVec3) -> Self {
        na::Vector3::new(v.x, v.y, v.z)
    }
}

impl From<na::Vector3<f32>> for DbVec3 {
    fn from(v: na::Vector3<f32>) -> Self {
        DbVec3 {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<DbQuat> for na::UnitQuaternion<f32> {
    fn from(q: DbQuat) -> Self {
        na::UnitQuaternion::from_quaternion(na::Quaternion::new(q.w, q.x, q.y, q.z))
    }
}

impl From<na::UnitQuaternion<f32>> for DbQuat {
    fn from(q: na::UnitQuaternion<f32>) -> Self {
        let quat = q.into_inner();
        DbQuat {
            x: quat.i,
            y: quat.j,
            z: quat.k,
            w: quat.w,
        }
    }
}

impl Default for DbQuat {
    fn default() -> Self {
        Self {
            x: 0.,
            y: -1., // Default for bevy app
            z: 0.,
            w: 0.,
        }
    }
}

/// A 3-dimensional vector.
#[derive(SpacetimeType, Clone, Copy, PartialEq)]
pub struct DbVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for DbVec3 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl DbVec3 {
    pub const ONE: Self = Self::new(1., 1., 1.);
    pub const ZERO: Self = Self::new(0., 0., 0.);

    /// Creates a new vector.
    #[inline(always)]
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Converts `self` to `[x, z]`, dropping the y component
    #[inline]
    #[must_use]
    pub const fn to_2d_array(&self) -> [f32; 2] {
        [self.x, self.z]
    }
}
#[derive(SpacetimeType, Clone, Copy, PartialEq)]
pub struct DbCapsule {
    pub radius: f32,
    pub half_height: f32,
}

#[derive(SpacetimeType, PartialEq)]
pub enum ColliderShape {
    Plane(f32),
    Cuboid(DbVec3),
    Capsule(DbCapsule),
}

#[derive(SpacetimeType, PartialEq)]
pub enum MoveIntent {
    /// Move along this path across frames until all points are reached
    Path(Vec<DbVec3>),
    /// Follow this actor across frames until it is reached, timeout, or too far.
    Actor(u64),
    /// Move toward this point (direction) for a single frame
    Point(DbVec3),
    /// No movement, idling
    None,
}

#[derive(SpacetimeType, PartialEq)]
pub enum ActorKind {
    Player(Identity),
    Monster(u32),
}

#[table(name = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,

    #[index(btree)]
    pub actor_id: Option<u64>,

    /// Persisted data to rebuild actor from
    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
    pub movement_speed: f32,
    pub grounded: bool,
}

#[table(name = projectile, public)]
pub struct Projectile {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    /// The actor that initiated this projectile
    #[index(btree)]
    pub source_actor_id: Option<u64>,

    /// When homing, this is the target
    #[index(btree)]
    pub target_actor_id: Option<u64>,

    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,

    pub speed: f32,

    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}

#[table(name = actor, public)]
pub struct Actor {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    pub kind: ActorKind,
    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
    // Per-actor movement tuning
    pub movement_speed: f32,
    // Ground contact state
    pub grounded: bool,

    pub move_intent: MoveIntent,
}
#[table(name = world_static, public)]
pub struct WorldStatic {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,

    pub shape: ColliderShape,
}

/// The HZ (FPS) at which the server should tick for movement.
const TICK_RATE: i64 = 60;
const DELTA_MICRO_SECS: i64 = 1_000_000 / TICK_RATE;
/// Constant downward fall speed (m/s) applied when not grounded.
const FALL_SPEED_MPS: f32 = -10.0;
/// Global collision skin distance (meters).
const SKIN: f32 = 0.02;
/// Global maximum snap distance to ground (meters).
const SNAP_MAX_DISTANCE: f32 = 0.3;
/// Global hover height above ground when snapped (meters).
const SNAP_HOVER_HEIGHT: f32 = 0.02;
/// Static accelerator for immutable world statics.
static WORLD_ACCEL: OnceLock<shared::collision::broad::WorldAccel> = OnceLock::new();

#[table(name = tick_timer, scheduled(tick))]
struct TickTimer {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,

    /// Used to compute delta time on server
    last_tick: Timestamp,
}

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    let tick_interval = TimeDuration::from_micros(DELTA_MICRO_SECS);
    ctx.db.tick_timer().scheduled_id().delete(1);
    ctx.db.tick_timer().insert(TickTimer {
        scheduled_id: 1,
        scheduled_at: spacetimedb::ScheduleAt::Interval(tick_interval),
        last_tick: ctx.timestamp,
    });

    // Seed world statics: ground plane and a test cuboid
    ctx.db.world_static().insert(WorldStatic {
        id: 0,
        translation: DbVec3::new(0.0, 0.0, 0.0),
        rotation: DbQuat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        scale: DbVec3::new(10.0, 1.0, 10.0), // visual size; physics uses plane
        shape: ColliderShape::Plane(0.0),
    });
    ctx.db.world_static().insert(WorldStatic {
        id: 0,
        translation: DbVec3::new(3.0, 1.0, 0.0),
        rotation: DbQuat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        scale: DbVec3::new(1.0, 1.0, 1.0),
        shape: ColliderShape::Cuboid(DbVec3::new(0.1, 1.0, 2.0)),
    });
    // Build world statics (immutable in this project).
    let statics = world_statics_to_shared(ctx);
    // Build and cache the world accelerator once.
    let accel = shared::collision::broad::build_world_accel(&statics);
    let _ = WORLD_ACCEL.set(accel);
}
fn world_statics_to_shared(ctx: &ReducerContext) -> Vec<shared::collision::StaticShape> {
    let mut out = Vec::new();
    for s in ctx.db.world_static().iter() {
        let t: na::Vector3<f32> = s.translation.into();
        let q: na::UnitQuaternion<f32> = s.rotation.into();
        let sc: na::Vector3<f32> = s.scale.into();

        match s.shape {
            ColliderShape::Plane(offset_along_normal) => {
                out.push(shared::collision::plane_from_pose(
                    q,
                    t,
                    offset_along_normal,
                ));
            }
            ColliderShape::Cuboid(half_extents) => {
                let he: na::Vector3<f32> = half_extents.into();
                let he_final = he.component_mul(&sc);
                out.push(shared::collision::cuboid_from_pose(he_final, t, q));
            }
            ColliderShape::Capsule(_) => {
                // Not used for world statics right now; ignore or handle if added later.
            }
        }
    }
    out
}
#[reducer]
fn tick(ctx: &ReducerContext, mut timer: TickTimer) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("`movement_tick` may not be invoked by clients.".into());
    }

    // Compute delta time in seconds and update the last_tick with the current Timestamp
    let delta_time_seconds = ctx
        .timestamp
        .time_duration_since(timer.last_tick)
        .unwrap_or(TimeDuration::from_micros(DELTA_MICRO_SECS))
        .to_micros() as f32
        / 1_000_000.0;
    timer.last_tick = ctx.timestamp;
    ctx.db.tick_timer().scheduled_id().update(timer);

    // Build statics for collision
    let statics = world_statics_to_shared(ctx);
    let accel = WORLD_ACCEL.get().expect("world accel not initialized");

    // Process entity's movement for those that have intent to move
    for mut source_actor in ctx.db.actor().iter() {
        match source_actor.move_intent {
            MoveIntent::None => {
                // Apply gravity-only step when there is no movement intent.
                let capsule = shared::collision::CapsuleSpec {
                    radius: source_actor.capsule_radius,
                    half_height: source_actor.capsule_half_height,
                };
                let start_pos = na::Vector3::new(
                    source_actor.translation.x,
                    source_actor.translation.y,
                    source_actor.translation.z,
                );
                // Downward motion for this tick.
                let fall_desired = na::Vector3::new(0.0, FALL_SPEED_MPS * delta_time_seconds, 0.0);
                let fall_req = shared::collision::kinematic::MoveRequest {
                    start_pos,
                    desired_translation: fall_desired,
                    capsule,
                    skin: SKIN,
                    max_iterations: 4,
                };
                let fall_col =
                    shared::collision::move_capsule_kinematic_with_accel(&statics, accel, fall_req);
                // Snap to ground if close enough.
                let (snapped_pos, hit) = shared::collision::snap_capsule_to_ground_with_accel(
                    &statics,
                    accel,
                    capsule,
                    fall_col.end_pos,
                    SNAP_MAX_DISTANCE,
                    SNAP_HOVER_HEIGHT,
                );
                source_actor.grounded = hit;
                let final_pos = if hit { snapped_pos } else { fall_col.end_pos };
                source_actor.translation.x = final_pos.x;
                source_actor.translation.y = final_pos.y;
                source_actor.translation.z = final_pos.z;

                ctx.db.actor().id().update(source_actor);
                continue;
            }
            MoveIntent::Point(point) => {
                // If already falling, cancel intent and apply gravity only.
                if !source_actor.grounded {
                    source_actor.move_intent = MoveIntent::None;

                    let capsule = shared::collision::CapsuleSpec {
                        radius: source_actor.capsule_radius,
                        half_height: source_actor.capsule_half_height,
                    };
                    let start_pos = na::Vector3::new(
                        source_actor.translation.x,
                        source_actor.translation.y,
                        source_actor.translation.z,
                    );
                    let fall_desired =
                        na::Vector3::new(0.0, FALL_SPEED_MPS * delta_time_seconds, 0.0);
                    let fall_req = shared::collision::kinematic::MoveRequest {
                        start_pos,
                        desired_translation: fall_desired,
                        capsule,
                        skin: SKIN,
                        max_iterations: 4,
                    };
                    let fall_col = shared::collision::move_capsule_kinematic_with_accel(
                        &statics, accel, fall_req,
                    );
                    let (snapped_pos, hit) = shared::collision::snap_capsule_to_ground_with_accel(
                        &statics,
                        accel,
                        capsule,
                        fall_col.end_pos,
                        SNAP_MAX_DISTANCE,
                        SNAP_HOVER_HEIGHT,
                    );
                    source_actor.grounded = hit;
                    let final_pos = if hit { snapped_pos } else { fall_col.end_pos };
                    source_actor.translation.x = final_pos.x;
                    source_actor.translation.y = final_pos.y;
                    source_actor.translation.z = final_pos.z;

                    ctx.db.actor().id().update(source_actor);
                    continue;
                }
                // 1) Compute desired 3D translation toward the target using a capsule-based acceptance radius.
                let current = na::Point3::new(
                    source_actor.translation.x,
                    source_actor.translation.y,
                    source_actor.translation.z,
                );
                // Constrain target.y to current.y (planar navigation), then zero out vertical movement.
                let target = na::Point3::new(point.x, current.y, point.z);
                let move_plan = shared::motion::compute_desired_with_capsule_acceptance(
                    current,
                    target,
                    source_actor.movement_speed, // movement speed (m/s)
                    delta_time_seconds,          // dt
                    source_actor.capsule_radius,
                );
                let desired = na::Vector3::new(
                    move_plan.desired_translation.x,
                    0.0,
                    move_plan.desired_translation.z,
                );

                // 2) Update yaw if we plan any horizontal movement this tick.
                let dx = desired.x;
                let dz = desired.z;
                let moved_sq = dx * dx + dz * dz;
                if moved_sq > f32::EPSILON {
                    // Yaw such that 0 faces -Z in client visuals (eyes point along -Z)
                    let yaw = (-dx).atan2(-dz);
                    source_actor.rotation = yaw_to_db_quat(yaw);
                }

                // 3) Commit translation via kinematic sweep-and-slide against statics.
                let start_pos = na::Vector3::new(current.x, current.y, current.z);
                let capsule = shared::collision::CapsuleSpec {
                    radius: source_actor.capsule_radius,
                    half_height: source_actor.capsule_half_height,
                };
                let move_req = shared::collision::kinematic::MoveRequest {
                    start_pos,
                    desired_translation: desired,
                    capsule,
                    skin: SKIN,
                    max_iterations: 4,
                };
                let col =
                    shared::collision::move_capsule_kinematic_with_accel(&statics, accel, move_req);

                let after_horizontal = col.end_pos;
                // First ground snap after horizontal movement
                let (snapped1_pos, hit1) = shared::collision::snap_capsule_to_ground_with_accel(
                    &statics,
                    accel,
                    capsule,
                    after_horizontal,
                    SNAP_MAX_DISTANCE,
                    SNAP_HOVER_HEIGHT,
                );
                if !hit1 {
                    // Apply constant downward velocity when not grounded
                    let fall_desired =
                        na::Vector3::new(0.0, FALL_SPEED_MPS * delta_time_seconds, 0.0);
                    let fall_req = shared::collision::kinematic::MoveRequest {
                        start_pos: after_horizontal,
                        desired_translation: fall_desired,
                        capsule,
                        skin: SKIN,
                        max_iterations: 4,
                    };
                    let fall_col = shared::collision::move_capsule_kinematic_with_accel(
                        &statics, accel, fall_req,
                    );
                    // Final snap after fall
                    let (snapped2_pos, hit2) = shared::collision::snap_capsule_to_ground_with_accel(
                        &statics,
                        accel,
                        capsule,
                        fall_col.end_pos,
                        SNAP_MAX_DISTANCE,
                        SNAP_HOVER_HEIGHT,
                    );
                    source_actor.grounded = hit2;
                    let final_pos = if hit2 { snapped2_pos } else { fall_col.end_pos };
                    source_actor.translation.x = final_pos.x;
                    source_actor.translation.y = final_pos.y;
                    source_actor.translation.z = final_pos.z;
                } else {
                    source_actor.grounded = true;
                    source_actor.translation.x = snapped1_pos.x;
                    source_actor.translation.y = snapped1_pos.y;
                    source_actor.translation.z = snapped1_pos.z;
                }

                if move_plan.finished {
                    source_actor.move_intent = MoveIntent::None;
                }

                ctx.db.actor().id().update(source_actor);
            }
            _ => {
                unimplemented!("todo");
            }
        }
    }

    Ok(())
}

#[reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);
    if let Some(player) = ctx.db.player().identity().find(ctx.sender) {
        // Clear any dangling actor reference on reconnect.
        ctx.db.player().identity().update(Player {
            actor_id: None,
            ..player
        });
    } else {
        // Create a player with sensible defaults so we can rebuild an actor later.
        ctx.db.player().insert(Player {
            identity: ctx.sender,
            actor_id: None,
            translation: DbVec3::new(0., 3.85, 0.),
            rotation: DbQuat::default(),
            scale: DbVec3::ONE,
            capsule_radius: 0.35,
            capsule_half_height: 0.75,
            movement_speed: 5.0,
            grounded: false,
        });
    }
}

#[reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    log::info!("Client disconnected: {:?}", ctx.sender);

    let Some(mut player) = ctx.db.player().identity().find(ctx.sender) else {
        return;
    };

    if let Some(actor_id) = player.actor_id {
        if let Some(actor) = ctx.db.actor().id().find(actor_id) {
            // Persist actor state back to player row.
            player.translation = actor.translation;
            player.rotation = actor.rotation;
            player.scale = actor.scale;
            player.capsule_radius = actor.capsule_radius;
            player.capsule_half_height = actor.capsule_half_height;
            player.movement_speed = actor.movement_speed;
            player.grounded = actor.grounded;

            // Delete actor and clear reference.
            ctx.db.actor().id().delete(actor.id);
            player.actor_id = None;
            ctx.db.player().identity().update(player);
            return;
        }
    }
}

#[reducer]
pub fn enter_world(ctx: &ReducerContext) {
    let Some(mut player) = ctx.db.player().identity().find(ctx.sender) else {
        log::error!("Player not found when trying to enter world");
        return;
    };

    if let Some(_) = player.actor_id {
        log::error!("Cannot enter the world twice");
        return;
    };

    // Rebuild actor directly from persisted player data.
    let actor = ctx.db.actor().insert(Actor {
        id: 0,
        scale: player.scale,
        kind: ActorKind::Player(player.identity),
        rotation: player.rotation,
        translation: player.translation,
        capsule_radius: player.capsule_radius,
        capsule_half_height: player.capsule_half_height,
        movement_speed: player.movement_speed,
        grounded: player.grounded,
        move_intent: MoveIntent::None,
    });

    player.actor_id = Some(actor.id);
    ctx.db.player().identity().update(player);
    log::info!("Client entered world: {:?}", ctx.sender);
}

#[spacetimedb::reducer]
pub fn leave_world(ctx: &ReducerContext) {
    let Some(mut player) = ctx.db.player().identity().find(ctx.sender) else {
        log::error!("Player not found when trying to leave world");
        return;
    };

    let Some(actor_id) = player.actor_id else {
        log::warn!("Player doesn't have an actor in game, cannot leave world");
        return;
    };
    let Some(actor) = ctx.db.actor().id().find(actor_id) else {
        log::warn!("Actor not found when trying to leave world");
        return;
    };

    // Persist actor state to player.
    player.translation = actor.translation;
    player.rotation = actor.rotation;
    player.scale = actor.scale;
    player.capsule_radius = actor.capsule_radius;
    player.capsule_half_height = actor.capsule_half_height;
    player.movement_speed = actor.movement_speed;
    player.grounded = actor.grounded;

    // Remove actor and clear reference.
    player.actor_id = None;
    ctx.db.actor().id().delete(actor.id);
    ctx.db.player().identity().update(player);

    log::info!("Client left world: {:?}", ctx.sender);
}

#[reducer]
pub fn request_move(ctx: &ReducerContext, intent: MoveIntent) -> Result<(), String> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        log::warn!(
            "Sender {:?} tried to request move without a valid Player entry.",
            ctx.sender
        );
        return Err("Player not found".to_string());
    };

    let Some(actor_id) = player.actor_id else {
        log::warn!(
            "Player {:?} tried to request move without entering world - no valid actor",
            ctx.sender
        );
        return Err("Actor not found".to_string());
    };
    let Some(mut actor) = ctx.db.actor().id().find(actor_id) else {
        log::warn!(
            "Player {:?} tried to request move without entering world - no valid actor",
            ctx.sender
        );
        return Err("Actor not found".to_string());
    };

    if !actor.grounded {
        return Err("Actor is falling; cannot set move intent right now".to_string());
    }

    actor.move_intent = intent;
    ctx.db.actor().id().update(actor);

    Ok(())
}
