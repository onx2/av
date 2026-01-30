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

// #[derive(SpacetimeType, Debug)]
// pub struct ActiveCharacterRow {
//     pub owner: Owner,
// }
// /// Finds the active character for this player
// /// Primary key of `Owner`
// #[spacetimedb::view(name = active_character_view, public)]
// pub fn active_character_view(ctx: &ViewContext) -> Option<ActiveCharacterRow> {
//     ctx.db
//         .active_character_tbl()
//         .identity()
//         .find(ctx.sender)
//         .map(|ac| ActiveCharacterRow { owner: ac.owner })
// let Some(cell_id) = ctx
//     .db
//     .movement_state_tbl()
//     .owner()
//     .find(&active_character.owner)
//     .map(|row| row.cell_id)
// else {
//     return vec![];
// };

// get_aoi_block(cell_id)
//     .into_iter()
//     .flat_map(|cell_id| ctx.db.movement_state_tbl().cell_id().filter(cell_id))
//     .filter_map(|ms| ActiveCharacterView { owner: ms.owner })
//     .collect()
// }
