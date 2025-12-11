use glam::Quat;
use shared::*;
use spacetimedb::*;

#[derive(SpacetimeType, Clone, Copy, PartialEq)]
pub struct DbQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl From<Quat> for DbQuat {
    fn from(quat: Quat) -> Self {
        Self {
            x: quat.x,
            y: quat.y,
            z: quat.z,
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

#[spacetimedb::table(name = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,

    #[index(btree)]
    pub actor_id: Option<u64>,
}

#[spacetimedb::table(name = actor, public)]
pub struct Actor {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    pub kind: ActorKind,
    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,

    pub move_intent: MoveIntent,
}

/// The HZ (FPS) at which the server should tick for movement.
const TICK_RATE: i64 = 60;
const DELTA_MICRO_SECS: i64 = 1_000_000 / TICK_RATE;

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

    // Process entity's movement for those that have intent to move
    for mut source_actor in ctx
        .db
        .actor()
        .iter()
        .filter(|pm| pm.move_intent != MoveIntent::None)
    {
        match source_actor.move_intent {
            MoveIntent::None => {
                log::error!("Player movement intent is None but should have been filtered out");
                continue;
            }
            MoveIntent::Point(point) => {
                let result = calculate_step_2d(CalculateStepArgs {
                    current_position: source_actor.translation.to_2d_array(),
                    target_position: point.to_2d_array(),
                    acceptance_radius: 0.25,
                    movement_speed: 5.0,
                    delta_time_seconds,
                });

                let dx = result.new_position[0] - source_actor.translation.x;
                let dz = result.new_position[1] - source_actor.translation.z;
                let moved_sq = dx * dx + dz * dz;
                if moved_sq > f32::EPSILON {
                    // Yaw such that 0 faces -Z in client visuals (eyes point along -Z)
                    source_actor.rotation = Quat::from_rotation_y((-dx).atan2(-dz)).into();
                }

                // Commit translation after computing delta
                source_actor.translation.x = result.new_position[0];
                source_actor.translation.z = result.new_position[1];

                if result.movement_finished {
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
        ctx.db.player().identity().update(Player {
            actor_id: None,
            ..player
        });
    } else {
        ctx.db.player().insert(Player {
            identity: ctx.sender,
            actor_id: None,
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

    let actor = ctx.db.actor().insert(Actor {
        id: 0,
        scale: DbVec3::ONE,
        kind: ActorKind::Player(player.identity),
        rotation: DbQuat::default(),
        translation: DbVec3::new(0., 1.5, 0.),
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

    actor.move_intent = intent;
    ctx.db.actor().id().update(actor);

    Ok(())
}
