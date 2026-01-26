use shared::OwnerId;
use spacetimedb::{table, Identity};

/// Main persistence table a person's "account"
#[table(name=player_tbl)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,

    pub owner_id: Option<OwnerId>,
}
