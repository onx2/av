use crate::types::*;
use shared::utils::get_aoi_block;
use spacetimedb::*;

/// Aggregated timing statistics for reducers/spans over time.
///
/// This is intended for lightweight, always-on profiling via sampling:
/// - `event` is typically the reducer name (e.g. "movement_tick").
/// - `span` is either "event" (whole reducer) or a named section (e.g. "kcc_move_shape").
/// - `key` is a stable unique identifier: "{event}::{span}".
///
/// Notes:
/// - Durations are stored in microseconds as integers to avoid float drift.
/// - `ema_us` is a best-effort exponential moving average in microseconds.
/// - You can periodically query/log these rows to identify hotspots.
#[table(name = timing_stats)]
pub struct TimingStats {
    /// Unique key per (event, span) pair.
    #[primary_key]
    pub key: String,

    /// Reducer/event name.
    #[index(btree)]
    pub event: String,

    /// Span name (or "event").
    #[index(btree)]
    pub span: String,

    /// Number of samples aggregated.
    pub samples: u64,

    /// Sum of durations (microseconds).
    pub total_us: u128,

    /// Running min/max duration in microseconds.
    pub min_us: u64,
    pub max_us: u64,

    /// Exponential moving average in microseconds (best-effort).
    pub ema_us: f64,

    /// Timestamp of the last update.
    pub last_updated_at: Timestamp,
}

/// Player account data persisted across sessions.
///
/// This table persists the last known actor state so players can rejoin
/// with the same parameters and location. The authoritative actor entity
/// is created/destroyed on demand and references back to this row.
#[table(name = player, public)]
pub struct Player {
    /// Unique identity (primary key).
    #[primary_key]
    pub identity: Identity,

    pub primary_stats_id: u32,
    pub secondary_stats_id: u32,
    pub vital_stats_id: u32,
    pub transform_data_id: u32,
    pub movement_data_id: u32,

    /// Optional live actor id. None if not currently in-world.
    #[index(btree)]
    pub actor_id: Option<u64>,

    /// Capsule radius used by the actor's kinematic collider (meters).
    pub capsule_radius: f32,
    /// Capsule half-height used by the actor's kinematic collider (meters).
    pub capsule_half_height: f32,
}

/// Live actor entity driven by the server's kinematic controller.
///
/// An `Actor` exists only while the player is "in world". The authoritative
/// values here are updated every tick by the server and may be mirrored
/// back to the `Player` row when leaving or disconnecting.
#[table(name = actor, index(name=should_move_and_is_player, btree(columns=[should_move, is_player])))]
pub struct Actor {
    /// Auto-incremented unique id (primary key).
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    pub primary_stats_id: u32,
    pub secondary_stats_id: u32,
    pub vital_stats_id: u32,

    #[unique]
    pub movement_data_id: u32,

    #[unique]
    pub transform_data_id: u32,

    // pub kind: ActorKind,
    /// An optional player identity when this actor is controlled, NOT a server actor.
    pub identity: Option<Identity>,
    /// Used alongside identity for faster btree lookups
    #[index(btree)]
    pub is_player: bool,

    #[index(btree)]
    pub should_move: bool,

    #[index(btree)]
    pub cell_id: u32,

    /// Capsule collider parameters (meters).
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}

#[derive(Default)]
#[table(name = transform_data)]
pub struct TransformData {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    /// Mixed-precision translation optimized for replication/storage:
    /// - `x`/`z` are stored as `f32` meters (full precision)
    /// - `y` is stored as quantized `i16` (0.1m per unit)
    pub translation: DbVec3i16,

    /// Quantized yaw (radians) stored as a single byte.
    /// Convention: `0..=u8::MAX` maps uniformly onto `[0, 2π)`.
    pub yaw: u8,
}

#[table(name = movement_data)]
pub struct MovementData {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    #[index(btree)]
    pub should_move: bool,

    /// Current movement intent.
    pub move_intent: MoveIntent,

    /// Whether the Actor was grounded last X grounded_grace_steps ago
    pub grounded: bool,

    /// The number of steps to wait before flipping grounded state
    pub grounded_grace_steps: u8,
}

#[table(name = primary_stats)]
pub struct PrimaryStats {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    pub strength: u8,
    pub dexterity: u8,
    pub fortitude: u8,
    pub intelligence: u8,
    pub piety: u8,
}

#[table(name = secondary_stats)]
pub struct SecondaryStats {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    /// Nominal horizontal movement speed (m/s), computed value
    pub movement_speed: f32,

    pub max_health: u16,
    pub max_mana: u16,
    pub max_stamina: u16,
}

impl Default for SecondaryStats {
    fn default() -> Self {
        Self {
            id: 0,
            movement_speed: 0.0,
            max_health: 0,
            max_mana: 0,
            max_stamina: 0,
        }
    }
}

#[table(name = vital_stats)]
pub struct VitalStats {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    pub health: u16,
    pub mana: u16,
    pub stamina: u16,
}

/// NOTE: `FakeWanderTickTimer` was moved out of `schema.rs` into the reducer module
/// (`server/src/reducers/spawn_fake.rs`) so the scheduled reducer symbol is in scope.
///
/// If you remove the fake-wander system later, delete:
/// - the timer table in `spawn_fake.rs`
/// - `FakeWanderState` (above)
/// - the reducers in `spawn_fake.rs`

/// Kinematic Character Controller (KCC) settings shared by server and clients.
///
/// This is intended to be a single-row table (e.g. `id = 1`) that both:
/// - the server reads to configure Rapier's `KinematicCharacterController`, and
/// - clients subscribe to in order to recreate the same controller configuration locally.
///
/// Notes
/// - Values are expressed in meters, seconds, and degrees (converted to radians at runtime).
/// - Autostep and snap-to-ground are always enabled (per current design).
#[table(name = kcc_settings, public)]
pub struct KccSettings {
    /// Unique id (primary key). Use a single row with `id = 1`.
    #[primary_key]
    pub id: u32,

    /// Small gap preserved between the character and its surroundings (meters).
    /// Keep `offset` small but non-zero for numerical stability
    pub offset: f32,

    /// Maximum climbable slope angle (degrees).
    pub max_slope_climb_deg: f32,

    /// Minimum slope angle (degrees) before automatic sliding starts.
    pub min_slope_slide_deg: f32,

    /// Autostep maximum height (meters). Always enabled.
    pub autostep_max_height: f32,

    /// Autostep minimum width (meters). Always enabled.
    pub autostep_min_width: f32,

    /// Whether the controller should slide against obstacles.
    pub slide: bool,

    /// Increase if the character gets stuck when sliding (small, meters).
    pub normal_nudge_factor: f32,

    /// Constant falling speed magnitude (m/s) applied as downward motion when airborne.
    pub fall_speed_mps: f32,

    /// Small downward bias magnitude (m/s) applied while grounded to satisfy snap-to-ground preconditions.
    pub grounded_down_bias_mps: f32,

    /// Probe distance (meters) used to detect whether the character is truly unsupported.
    ///
    /// If the actor is not grounded but still within the grounded grace period, a downward probe
    /// of this length can be used to decide whether to cancel the grace immediately (e.g. when
    /// stepping off a ledge) or keep it (e.g. stair edge flicker).
    pub hard_airborne_probe_distance: f32,

    /// The squared distance from a point that we consider reached / close enough
    /// Helpful to prevent floating point errors and jitter
    pub point_acceptance_radius_sq: f32,
}

/// Static collider rows used to build the immutable world collision geometry.
///
/// The server reads these rows into an in-memory Rapier query world once, and reuses it
/// every tick for scene queries and the kinematic character controller (KCC).
#[table(name = world_static, public)]
pub struct WorldStatic {
    /// Unique id (primary key).
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    /// World transform applied to the shape.
    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,

    /// Collider shape definition.
    pub shape: ColliderShape,
}

#[view(name = aoi_actor, public)]
fn aoi_actor_view(ctx: &ViewContext) -> Vec<AoiActor> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return Vec::new();
    };
    let Some(actor_id) = player.actor_id else {
        return Vec::new();
    };
    let Some(actor) = ctx.db.actor().id().find(actor_id) else {
        return Vec::new();
    };

    let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);

    aoi_block
        .into_iter()
        .flat_map(|cell_id| ctx.db.actor().cell_id().filter(cell_id))
        .map(|a| a.into())
        .collect()
}

#[view(name = aoi_transform_data, public)]
fn aoi_transform_data_view(ctx: &ViewContext) -> Vec<TransformData> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
        return Vec::new();
    };
    let Some(actor_id) = player.actor_id else {
        return Vec::new();
    };
    let Some(actor) = ctx.db.actor().id().find(actor_id) else {
        return Vec::new();
    };

    let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);
    aoi_block
        .into_iter()
        .flat_map(|cell_id| {
            ctx.db.actor().cell_id().filter(cell_id).map(|actor| {
                ctx.db
                    .transform_data()
                    .id()
                    .find(actor.transform_data_id)
                    .unwrap_or_default()
            })
        })
        .collect()
}
