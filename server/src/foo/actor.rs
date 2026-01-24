use shared::owner::Owner;
use spacetimedb::table;

/// Ephemeral
///
/// In-game, ephemeral, representation of a player's Character, a NPC, or a Monster.
///
/// Right now I'm thinking this is essentially a marker row for when something is spawned into the world with all its data.
/// TBD though on what this should hold... Might remove it entirely in favor of individual private tables and public views.
#[table(name=actor_tbl)]
pub struct Actor {
    #[primary_key]
    pub owner: Owner,
}
