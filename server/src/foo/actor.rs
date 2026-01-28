use shared::Owner;
use spacetimedb::table;

/// Ephemeral
///
/// In-game, ephemeral, representation of a player's Character, a NPC, or a Monster.
/// This is a marker row for when something is spawned into the world with all its data.
#[table(name=actor_tbl)]
pub struct Actor {
    #[primary_key]
    pub owner: Owner,

    #[index(btree)]
    pub cell_id: u32,
}
