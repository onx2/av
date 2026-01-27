use num_traits::Zero;
use shared::bitmask_flags::{BitmaskFlags, FlagBitmask, FlagsContainer};
use shared::{define_bitmask_flags, pack_owner, Owner, OwnerKind};
use spacetimedb::{reducer, table, ReducerContext, Table};

/// SpacetimeDB table that stores bitmask-backed gameplay flags per owner.
///
/// Storage is a primitive (`u64` via `shared::bitmask_flags::FlagsContainer`) so we avoid needing
/// `SpacetimeType` derives for the shared helper types.
#[table(name=gameplay_flags_tbl)]
pub struct GameplayFlags {
    #[primary_key]
    pub owner: Owner,

    /// Raw bitmask. Interpret and manipulate using `shared::bitmask_flags::BitmaskFlags`.
    pub data: FlagsContainer,
}

impl GameplayFlags {
    /// Adds a single flag.
    pub fn add_tag<U: FlagBitmask<Storage = FlagsContainer>>(&mut self, tag: U) {
        let mut tmp = BitmaskFlags::<FlagsContainer>::new(self.data);
        tmp.add(tag);
        self.data = tmp.bits;
    }

    /// Removes a single flag.
    pub fn remove_tag<U: FlagBitmask<Storage = FlagsContainer>>(&mut self, tag: U) {
        let mut tmp = BitmaskFlags::<FlagsContainer>::new(self.data);
        tmp.remove(tag);
        self.data = tmp.bits;
    }

    /// Returns true if the flag is set.
    pub fn has_tag<U: FlagBitmask<Storage = FlagsContainer>>(&self, tag: U) -> bool {
        BitmaskFlags::<FlagsContainer>::new(self.data).has(tag)
    }

    /// Adds all flags in `tags`.
    pub fn add_many_tags<U: FlagBitmask<Storage = FlagsContainer> + Copy>(&mut self, tags: &[U]) {
        let mut tmp = BitmaskFlags::<FlagsContainer>::new(self.data);
        tmp.add_many(tags);
        self.data = tmp.bits;
    }

    /// Removes all flags in `tags`.
    pub fn remove_many_tags<U: FlagBitmask<Storage = FlagsContainer> + Copy>(
        &mut self,
        tags: &[U],
    ) {
        let mut tmp = BitmaskFlags::<FlagsContainer>::new(self.data);
        tmp.remove_many(tags);
        self.data = tmp.bits;
    }

    /// Returns true if *all* flags in `tags` are set. Empty slice => true.
    pub fn has_all_tags<U: FlagBitmask<Storage = FlagsContainer> + Copy>(
        &self,
        tags: &[U],
    ) -> bool {
        BitmaskFlags::<FlagsContainer>::new(self.data).has_all(tags)
    }

    /// Returns true if *any* flag in `tags` is set. Empty slice => false.
    pub fn has_any_tags<U: FlagBitmask<Storage = FlagsContainer> + Copy>(
        &self,
        tags: &[U],
    ) -> bool {
        BitmaskFlags::<FlagsContainer>::new(self.data).has_any(tags)
    }

    /// Clears all flags.
    pub fn clear_tags(&mut self) {
        self.data = FlagsContainer::zero();
    }
}

#[reducer]
pub fn foobar(ctx: &ReducerContext) {
    let Some(mut tags) = ctx
        .db
        .gameplay_flags_tbl()
        .owner()
        .find(pack_owner(1, OwnerKind::Character))
    else {
        log::error!("No tags found for owner 1");
        return;
    };
    log::info!("Tags data before: {:?}", tags.data);

    tags.add_tag(UnitStatus::InCombat);
    tags.add_tag(UnitStatus::IsFriendly);
    tags.add_tag(UnitStatus::Stunned);
    let data = tags.data.clone();
    ctx.db.gameplay_flags_tbl().owner().update(tags);
    log::info!("Tags data after: {:?}", data);
}

pub fn regenerate(ctx: &ReducerContext) {
    ctx.db.gameplay_flags_tbl().iter().for_each(|row| {
        ctx.db.gameplay_flags_tbl().delete(row);
    });

    ctx.db.gameplay_flags_tbl().insert(GameplayFlags {
        owner: pack_owner(1, OwnerKind::Character),
        data: FlagsContainer::zero(),
    });
}

define_bitmask_flags!(UnitStatus, u64, {
    IsFriendly,
    InCombat,
    Stunned,
    Burning,
    Slowed,
    Invulnerable,
});
