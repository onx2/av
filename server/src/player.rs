use spacetimedb::{table, Identity, Timestamp};

/// Main persistence table a person's "account"
#[table(name=player_tbl)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,

    pub last_login_at: Timestamp,

    #[index(btree)]
    pub online: bool,

    pub banned: bool,
}
