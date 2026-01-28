use shared::{pack_owner, AsOwner, Owner, OwnerId, OwnerKind};
use spacetimedb::table;

/// A spawned monster instance in the world.
///
/// This table exists to support **multiple monsters of the same type** being spawned at once.
/// The monster "type/definition" lives in `monster_tbl` (see `Monster.monster_id`).
///
/// - `owner_id` is an auto-incrementing instance id. It is packed into [`Owner`] using
///   [`OwnerKind::Monster`].
/// - `monster_id` points back to the monster definition/type in `monster_tbl`.
#[table(name=monster_instance_tbl)]
pub struct MonsterInstance {
    /// Unique id for this spawned monster instance (packed into [`Owner`]).
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    /// Monster definition/type id from `monster_tbl`.
    #[index(btree)]
    pub monster_id: u16,
}

impl AsOwner for MonsterInstance {
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
