use shared::{pack_owner, AsOwner, Owner, OwnerId, OwnerKind};
use spacetimedb::{table, ReducerContext, Table};

use crate::foo::{actor_tbl, Actor};

/// TODO: Monsters should maybe be different, not DataOwner impl but some partial amount of data like stats?
/// This could also just be generated too when they are spawned in based on some criteria or algorithm.
/// The persistence layer for the types of enemies that can be spawned into the world (Actor)
///
/// **Possible source of `owner` found in other tables.**
#[table(name=monster_tbl)]
pub struct Monster {
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    pub name: String,
}

impl AsOwner for Monster {
    fn owner(&self) -> Owner {
        pack_owner(self.owner_id, OwnerKind::Monster)
    }
    fn owner_id(&self) -> OwnerId {
        self.owner_id
    }
    fn owner_kind(&self) -> OwnerKind {
        OwnerKind::Monster
    }
}

impl Monster {
    pub fn spawn(&self, ctx: &ReducerContext) -> Result<(), String> {
        let owner = self.owner();

        // ctx.db.actor_tbl().insert(Actor { owner });
        // Transform::insert(ctx, owner, self.transform);
        // PrimaryStats::insert(ctx, owner, self.primary_stats);
        // Health::insert(ctx, owner, self.health);
        // Mana::insert(ctx, owner, self.mana);
        // Experience::insert(ctx, owner, self.experience);
        // Level::insert(ctx, owner, self.level);
        Ok(())
    }
}
