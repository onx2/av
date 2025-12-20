use crate::types::*;
use shared::utils::get_aoi_block;
use spacetimedb::*;

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

    /// Last known rotation of the actor (yaw).
    pub yaw: f32,

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
    pub translation: DbVec3, // 12 bytes
    // pub rotation: DbQuat, // 16 bytes
    pub yaw: f32, // 16 bytes
    // pub scale: DbVec3,    // 12 bytes
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

    #[index(btree)]
    pub cell_id: u32,
}

pub struct MyTable {
    // ... fields
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

#[client_visibility_filter]
const ACTOR_FILTER: Filter = Filter::Sql("SELECT * FROM actor_in_aoi WHERE identity = :sender");

#[table(name = actor_in_aoi, index(name = identity_actor, btree(columns = [identity, actor_id])), public)]
pub struct ActorInAoi {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub identity: Identity,
    pub actor_id: u64,
    pub translation: DbVec3,
    pub kind: ActorKind,
    pub yaw: f32,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}

// #[view(name = aoi_actors, public)]
// fn aoi_actors_view(ctx: &ViewContext) -> Vec<ActorInAoi> {
//     let Some(player) = ctx.db.player().identity().find(ctx.sender) else {
//         return Vec::new();
//     };
//     let Some(actor_id) = player.actor_id else {
//         return Vec::new();
//     };
//     let Some(actor) = ctx.db.actor().id().find(actor_id) else {
//         return Vec::new();
//     };

//     let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);

//     aoi_block
//         .into_iter()
//         // flat_map turns each cell_id lookup into a single stream of actors
//         .flat_map(|cell_id| ctx.db.actor().cell_id().filter(cell_id))
//         .map(|actor| ActorInAoi {
//             id: actor.id,
//             kind: actor.kind,
//             identity: ctx.sender,
//             translation: actor.translation,
//             yaw: actor.yaw,
//             capsule_radius: actor.capsule_radius,
//             capsule_half_height: actor.capsule_half_height,
//         })
//         .collect()
// }
