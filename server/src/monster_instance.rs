use shared::{pack_owner, AsOwner, Owner, OwnerId, OwnerKind};
use spacetimedb::{table, SpacetimeType};

/// A spawned monster instance in the world.
///
/// `owner_id` is packed into [`Owner`] using [`OwnerKind::Monster`].
///
/// This table exists to support multiple monsters of the same type being spawned at once.
/// The monster "type/definition" lives in `monster_tbl` (see `Monster.monster_id`).
///
/// - `owner_id` is an auto-incrementing instance id.
/// - `monster_id` points back to the monster definition/type in `monster_tbl`.
#[table(name=monster_instance_tbl)]
pub struct MonsterInstanceRow {
    /// Unique id for this spawned monster instance (packed into [`Owner`]).
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    /// Monster definition/type id from `monster_tbl`.
    #[index(btree)]
    pub monster_id: u16,
}

impl AsOwner for MonsterInstanceRow {
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

#[derive(SpacetimeType, Debug, Default, PartialEq, Clone, Copy)]
pub struct ActiveMonster {
    pub owner: Owner,
    pub monster_id: u16,
}

// #[spacetimedb::view(name = active_character_view, public)]
// pub fn active_character_view(ctx: &ViewContext) -> Vec<ActiveMonster> {
//     ctx.db.active_character_tbl().identity().find(ctx.sender)
// }
