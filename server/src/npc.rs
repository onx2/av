use shared::{pack_owner, AsOwner, Owner, OwnerId, OwnerKind};
use spacetimedb::table;

/// The persistence layer for the types of enemies that can be spawned into the world (Actor)
///
/// **Possible source of `owner` found in other tables.**
#[table(name=npc_tbl)]
pub struct NpcRow {
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    pub name: String,
}

impl AsOwner for NpcRow {
    fn owner(&self) -> Owner {
        pack_owner(self.owner_id, OwnerKind::Npc)
    }
    fn owner_id(&self) -> OwnerId {
        self.owner_id
    }
    fn owner_kind(&self) -> OwnerKind {
        OwnerKind::Npc
    }
}
