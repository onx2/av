use shared::Owner;
use spacetimedb::{table, Identity, SpacetimeType, ViewContext};

/// Marker table for the active character for a given player
#[table(name=active_character_tbl)]
pub struct ActiveCharacter {
    #[primary_key]
    pub identity: Identity,

    #[unique]
    pub owner: Owner,
}
impl ActiveCharacter {
    pub fn new(identity: Identity, owner: Owner) -> Self {
        Self { identity, owner }
    }
}

#[derive(SpacetimeType, Debug)]
pub struct ActiveCharacterRow {
    pub owner: Owner,
}
/// Finds the active character for this player
/// Primary key of `Owner`
#[spacetimedb::view(name = active_character_view, public)]
pub fn active_character_view(ctx: &ViewContext) -> Option<ActiveCharacterRow> {
    ctx.db
        .active_character_tbl()
        .identity()
        .find(ctx.sender)
        .map(|ac| ActiveCharacterRow { owner: ac.owner })
}
