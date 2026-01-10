use crate::types::*;
use spacetimedb::*;

/// Player account data persisted across sessions.
///
/// This table persists the last known actor state so players can rejoin
/// with the same parameters and location. The authoritative actor entity
/// is created/destroyed on demand and references back to this row.
#[table(name = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,

    pub primary_stats_id: u32,
    pub secondary_stats_id: u32,
    pub vital_stats_id: u32,

    pub transform_data_id: u64,

    /// Optional live actor id. None if not currently in-world.
    #[index(btree)]
    pub actor_id: Option<u64>,

    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}

/// Live actor entity driven by the server's kinematic controller.
///
/// An `Actor` exists only while the player is "in world". The authoritative
/// values here are updated every tick by the server and may be mirrored
/// back to the `Player` row when leaving or disconnecting.
#[table(name = actor, index(name=should_move_and_is_player, btree(columns=[should_move, is_player])))]
pub struct Actor {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    pub primary_stats_id: u32,
    pub secondary_stats_id: u32,
    pub vital_stats_id: u32,

    #[unique]
    pub transform_data_id: u64,

    /// An optional player identity when this actor is controlled, NOT a server actor.
    pub identity: Option<Identity>,

    /// Used alongside identity for faster btree lookups
    #[index(btree)]
    pub is_player: bool,

    #[index(btree)]
    pub should_move: bool,

    pub move_intent: MoveIntent,

    pub grounded: bool,

    #[index(btree)]
    pub cell_id: u32,

    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}

#[derive(Default)]
#[table(name = transform_data)]
pub struct TransformData {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    pub translation: DbVec3,

    /// Quantized yaw (radians) stored as a single byte.
    ///
    /// Convention: `0..=255` maps onto `[0, 2π)`.
    pub yaw: u8,
}

#[table(name = primary_stats, public)]
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

#[table(name = secondary_stats, public)]
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

#[table(name = vital_stats, public)]
pub struct VitalStats {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    pub health: u16,
    pub mana: u16,
    pub stamina: u16,
}

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
