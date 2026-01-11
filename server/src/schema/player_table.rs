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

    #[unique]
    pub primary_stats_id: u32,
    #[unique]
    pub secondary_stats_id: u32,
    #[unique]
    pub vital_stats_id: u32,
    #[unique]
    pub transform_data_id: u64,

    /// Optional live actor id. None if not currently in-world.
    #[index(btree)]
    pub actor_id: Option<u64>,

    pub capsule_radius: f32,
    pub capsule_half_height: f32,
}
