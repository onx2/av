use crate::model::{DbQuat, DbVec3};
use spacetimedb::*;

/// Capsule dimensions for collider definitions.
///
/// Semantics:
/// - `radius`: radius of spherical caps and cylinder.
/// - `half_height`: half of the cylinder length along local +Y.
/// - Total capsule height = `2*half_height + 2*radius`.
#[derive(SpacetimeType, Clone, Copy, PartialEq)]
pub struct DbCapsule {
    pub radius: f32,
    pub half_height: f32,
}

/// Collider shape used by world statics (and potentially triggers in the future).
///
/// Notes:
/// - Variants are newtype-like to keep storage compact and easy to serialize.
/// - Shapes are combined with per-row `translation`, `rotation`, and `scale`.
#[derive(SpacetimeType, PartialEq)]
pub enum ColliderShape {
    /// Infinite plane (half-space). `f32` is the offset along the plane normal:
    /// the plane satisfies `n ⋅ x = dist`, where `n = rotation * +Y`.
    Plane(f32),

    /// Oriented box defined by local half-extents (hx, hy, hz).
    /// The final physics size used by the server is `half_extents * scale`.
    Cuboid(DbVec3),

    /// Y-aligned capsule with `radius` and `half_height`.
    Capsule(DbCapsule),
}

/// Movement intent for an actor.
///
/// Match arms are handled by the server's tick reducer; unsupported variants
/// can be extended in the future.
#[derive(SpacetimeType, PartialEq)]
pub enum MoveIntent {
    /// Follow a sequence of waypoints (in world space) across multiple frames.
    Path(Vec<DbVec3>),

    /// Follow a dynamic actor by id.
    Actor(u64),

    /// Move toward this point (direction) for a single frame.
    Point(DbVec3),

    /// No movement intent (idling).
    None,
}

/// Logical kind/ownership for an actor.
///
/// Extend as needed for NPCs, bosses, and other categories.
#[derive(SpacetimeType, PartialEq)]
pub enum ActorKind {
    /// A player-controlled actor keyed by the user's identity.
    Player(Identity),
    /// A simple monster/NPC variant.
    Monster(u32),
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

    /// Optional live actor id. None if not currently in-world.
    #[index(btree)]
    pub actor_id: Option<u64>,

    // ------ Persisted actor state (server-authoritative) ------
    /// Last known translation of the actor (meters).
    pub translation: DbVec3,
    /// Last known rotation of the actor (unit quaternion).
    pub rotation: DbQuat,
    /// Visual/logic scale of the actor (component-wise).
    pub scale: DbVec3,

    /// Capsule radius used by the actor's kinematic collider (meters).
    pub capsule_radius: f32,
    /// Capsule half-height used by the actor's kinematic collider (meters).
    pub capsule_half_height: f32,

    /// Nominal horizontal movement speed in meters/second.
    pub movement_speed: f32,
}

/// Live actor entity driven by the server's kinematic controller.
///
/// An `Actor` exists only while the player is "in world". The authoritative
/// values here are updated every tick by the server and may be mirrored
/// back to the `Player` row when leaving or disconnecting.
#[table(name = actor, public)]
pub struct Actor {
    /// Auto-incremented unique id (primary key).
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    /// Logical kind/ownership of this actor.
    pub kind: ActorKind,

    /// World transform (meters / unit quaternion).
    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,

    /// Capsule collider parameters (meters).
    pub capsule_radius: f32,
    pub capsule_half_height: f32,

    /// Nominal horizontal movement speed (m/s).
    pub movement_speed: f32,

    /// Current movement intent.
    pub move_intent: MoveIntent,

    /// Whether the Actor was grounded last X grounded_grace_steps ago
    pub grounded: bool,

    /// The number of steps to wait before flipping grounded state
    pub grounded_grace_steps: u8,
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

    /// Snap-to-ground distance threshold (meters). Always enabled.
    pub snap_to_ground: f32,

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
