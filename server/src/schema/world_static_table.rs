use crate::types::*;
use spacetimedb::*;

/// Static collider rows used to build the immutable world collision geometry.
///
/// The server reads these rows into an in-memory Rapier query world once, and reuses it
/// every tick for scene queries and the kinematic character controller (KCC).
#[table(name = world_static, public)]
pub struct WorldStatic {
    /// Unique id (primary key).
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    /// World transform applied to the shape.
    pub translation: DbVec3,
    pub rotation: DbQuat,
    pub scale: DbVec3,

    /// Collider shape definition.
    pub shape: ColliderShape,
}
