use shared::Owner;
use spacetimedb::{reducer, table, ReducerContext, Table};

#[derive(Debug)]
#[table(name=gameplay_tag_tbl)]
pub struct GameplayTag {
    #[auto_inc]
    #[primary_key]
    pub id: u16,

    // Damage.Magic.Fire
    #[index(btree)]
    pub path: String,
}

#[table(name=owner_gameplay_tag_tbl)]
pub struct OwnerGameplayTag {
    #[index(btree)]
    pub owner: Owner,

    // this could maybe be a string... so the tag directly so we can query
    // this table using filter("Damage.Physical"..) to find all owners with physics damage.
    #[index(btree)]
    pub tag: u16, // String

    pub count: u8,
}
impl OwnerGameplayTag {
    // pub fn add(ctx: &ReducerContext, owner: Owner, tag: String, count: u8) {
    //     ctx.db.owner_gameplay_tag_tbl().insert(OwnerGameplayTag {
    //         owner,
    //         tag,
    //         count,
    //     });
    // }
}

#[reducer]
pub fn foo(ctx: &ReducerContext) {
    for tag in ctx
        .db
        .gameplay_tag_tbl()
        .path()
        .filter("Damage.Physical."..)
    {
        log::info!("Tag: {:?}", tag);
    }
}

pub fn tag_regenerate(ctx: &ReducerContext) {
    ctx.db.gameplay_tag_tbl().iter().for_each(|row| {
        ctx.db.gameplay_tag_tbl().delete(row);
    });

    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage".to_string(),
    });
    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage.Magic".to_string(),
    });
    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage.Magic.Fire".to_string(),
    });
    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage.Magic.Arcane".to_string(),
    });
    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage.Physical".to_string(),
    });
    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage.Physical.Blunt".to_string(),
    });
    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage.Physical.Slash".to_string(),
    });
    ctx.db.gameplay_tag_tbl().insert(GameplayTag {
        id: 0,
        path: "Damage.Physical.Pierce".to_string(),
    });
}

// use num_traits::PrimInt;
// use shared::Owner;
// use spacetimedb::{table, SpacetimeType};

// pub type GameplayFlagsSize = u64;

// #[table(name=gameplay_flags_tbl)]
// pub struct GameplayFlags {
//     #[primary_key]
//     pub owner: Owner,

//     pub data: GameplayFlagsData<GameplayFlagsSize>,
// }

// impl GameplayFlags {
//     pub fn add_tag<U: FlagBitmask<Storage = GameplayFlagsSize>>(&mut self, tag: U) {
//         self.data.add(tag);
//     }

//     pub fn remove_tag<U: FlagBitmask<Storage = GameplayFlagsSize>>(&mut self, tag: U) {
//         self.data.remove(tag);
//     }

//     pub fn has_tag<U: FlagBitmask<Storage = GameplayFlagsSize>>(&self, tag: U) -> bool {
//         self.data.has(tag)
//     }

//     pub fn add_many_tags<U: FlagBitmask<Storage = GameplayFlagsSize> + Copy>(
//         &mut self,
//         tags: &[U],
//     ) {
//         self.data.add_many(tags);
//     }

//     pub fn remove_many_tags<U: FlagBitmask<Storage = GameplayFlagsSize> + Copy>(
//         &mut self,
//         tags: &[U],
//     ) {
//         self.data.remove_many(tags);
//     }

//     pub fn has_all_tags<U: FlagBitmask<Storage = GameplayFlagsSize> + Copy>(
//         &self,
//         tags: &[U],
//     ) -> bool {
//         self.data.has_all(tags)
//     }

//     pub fn has_any_tags<U: FlagBitmask<Storage = GameplayFlagsSize> + Copy>(
//         &self,
//         tags: &[U],
//     ) -> bool {
//         self.data.has_any(tags)
//     }

//     pub fn clear_tags(&mut self) {
//         self.data.clear();
//     }
// }

// /// The Database-compatible wrapper for our bitmask.
// #[derive(SpacetimeType, Default, Copy, Clone, Debug, PartialEq, Eq)]
// pub struct GameplayFlagsData<T: PrimInt> {
//     pub bits: T,
// }

// impl<T: PrimInt> GameplayFlagsData<T> {
//     pub fn new(initial: T) -> Self {
//         Self { bits: initial }
//     }

//     pub fn add<U: FlagBitmask<Storage = T>>(&mut self, tag: U) {
//         self.bits = self.bits | tag.mask();
//     }

//     pub fn remove<U: FlagBitmask<Storage = T>>(&mut self, tag: U) {
//         self.bits = self.bits & !tag.mask();
//     }

//     pub fn has<U: FlagBitmask<Storage = T>>(&self, tag: U) -> bool {
//         (self.bits & tag.mask()) != T::zero()
//     }

//     pub fn add_many<U: FlagBitmask<Storage = T> + Copy>(&mut self, tags: &[U]) {
//         for tag in tags {
//             self.add(*tag);
//         }
//     }

//     pub fn remove_many<U: FlagBitmask<Storage = T> + Copy>(&mut self, tags: &[U]) {
//         for tag in tags {
//             self.remove(*tag);
//         }
//     }

//     pub fn has_all<U: FlagBitmask<Storage = T> + Copy>(&self, tags: &[U]) -> bool {
//         if tags.is_empty() {
//             return true;
//         }
//         let combined = tags.iter().fold(T::zero(), |acc, t| acc | t.mask());
//         (self.bits & combined) == combined
//     }

//     pub fn has_any<U: FlagBitmask<Storage = T> + Copy>(&self, tags: &[U]) -> bool {
//         if tags.is_empty() {
//             return false;
//         }
//         let combined = tags.iter().fold(T::zero(), |acc, t| acc | t.mask());
//         (self.bits & combined) != T::zero()
//     }

//     pub fn clear(&mut self) {
//         self.bits = T::zero();
//     }
// }
