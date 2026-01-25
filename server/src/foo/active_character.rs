use shared::owner::Owner;
use spacetimedb::{table, Identity};

/// Marker table for the active character for a given player, prevent
#[table(name=active_character_tbl)]
pub struct ActiveCharacter {
    #[primary_key]
    pub identity: Identity,

    #[unique]
    pub owner: Owner,
}
