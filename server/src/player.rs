use crate::{character_instance_tbl, character_tbl};
use spacetimedb::{table, Identity, ReducerContext, Table, Timestamp};

/// Main persistence table a person's "account"
#[table(name=player_tbl)]
pub struct PlayerRow {
    #[primary_key]
    pub identity: Identity,

    pub last_login_at: Timestamp,

    #[index(btree)]
    pub online: bool,

    /// UNIMPLEMENTED: Whether this player is allowed to play the game
    pub banned: bool,
}

impl PlayerRow {
    pub fn connect(ctx: &ReducerContext) {
        if let Some(mut player) = ctx.db.player_tbl().identity().find(ctx.sender) {
            player.online = true;
            player.last_login_at = ctx.timestamp;
            ctx.db.player_tbl().identity().update(player);
        } else {
            ctx.db.player_tbl().insert(PlayerRow {
                identity: ctx.sender,
                last_login_at: ctx.timestamp,
                online: true,
                banned: false,
            });
        };
    }

    pub fn disconnect(ctx: &ReducerContext) {
        let Some(mut player) = ctx.db.player_tbl().identity().find(ctx.sender) else {
            log::error!("Disconnect: Unable to find player: {:?}", ctx.sender);
            return;
        };
        player.online = false;
        ctx.db.player_tbl().identity().update(player);

        let Some(ci) = ctx.db.character_instance_tbl().identity().find(ctx.sender) else {
            log::info!("Disconnect: Unable to find active char: {:?}", ctx.sender);
            return;
        };
        let Some(character) = ctx.db.character_tbl().id().find(ci.character_id) else {
            log::error!("Disconnect: Unable to find char: {:?}", ctx.sender);
            return;
        };

        character.leave_game(ctx);
    }
}
