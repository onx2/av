use shared::{pack_owner, AsOwner, Owner, OwnerId, OwnerKind};
use spacetimedb::table;

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
