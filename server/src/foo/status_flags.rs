use num_traits::Zero;
use shared::{define_bitmask_flags, BitmaskFlags, FlagBitmask, FlagsContainer, Owner};
use spacetimedb::{table, ReducerContext, Table};

/// SpacetimeDB table that stores bitmask-backed status flags per owner.
///
/// Storage is a primitive (`u64` via `shared::bitmask_flags::FlagsContainer`) so we avoid needing
/// `SpacetimeType` derives for the shared helper types.
///
/// This is meant to store things like Stunned, InCombat, for fast lookup/reference of the status of a given owner.
#[table(name=status_flags_tbl)]
pub struct StatusFlags {
    #[primary_key]
    pub owner: Owner,

    /// Raw bitmask. Interpret and manipulate using `shared::bitmask_flags::BitmaskFlags`.
    pub data: StatusFlagsData,
}
pub type StatusFlagsData = FlagsContainer;

impl StatusFlags {
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: StatusFlagsData) {
        ctx.db.status_flags_tbl().insert(Self { owner, data });
    }

    /// Adds a single flag.
    pub fn add_tag<U: FlagBitmask<Storage = StatusFlagsData>>(&mut self, tag: U) {
        let mut tmp = BitmaskFlags::<StatusFlagsData>::new(self.data);
        tmp.add(tag);
        self.data = tmp.bits;
    }

    /// Removes a single flag.
    pub fn remove_tag<U: FlagBitmask<Storage = StatusFlagsData>>(&mut self, tag: U) {
        let mut tmp = BitmaskFlags::<StatusFlagsData>::new(self.data);
        tmp.remove(tag);
        self.data = tmp.bits;
    }

    /// Returns true if the flag is set.
    pub fn has_tag<U: FlagBitmask<Storage = StatusFlagsData>>(&self, tag: U) -> bool {
        BitmaskFlags::<StatusFlagsData>::new(self.data).has(tag)
    }

    /// Adds all flags in `tags`.
    pub fn add_many_tags<U: FlagBitmask<Storage = StatusFlagsData> + Copy>(&mut self, tags: &[U]) {
        let mut tmp = BitmaskFlags::<StatusFlagsData>::new(self.data);
        tmp.add_many(tags);
        self.data = tmp.bits;
    }

    /// Removes all flags in `tags`.
    pub fn remove_many_tags<U: FlagBitmask<Storage = StatusFlagsData> + Copy>(
        &mut self,
        tags: &[U],
    ) {
        let mut tmp = BitmaskFlags::<StatusFlagsData>::new(self.data);
        tmp.remove_many(tags);
        self.data = tmp.bits;
    }

    /// Returns true if *all* flags in `tags` are set. Empty slice => true.
    pub fn has_all_tags<U: FlagBitmask<Storage = StatusFlagsData> + Copy>(
        &self,
        tags: &[U],
    ) -> bool {
        BitmaskFlags::<StatusFlagsData>::new(self.data).has_all(tags)
    }

    /// Returns true if *any* flag in `tags` is set. Empty slice => false.
    pub fn has_any_tags<U: FlagBitmask<Storage = StatusFlagsData> + Copy>(
        &self,
        tags: &[U],
    ) -> bool {
        BitmaskFlags::<StatusFlagsData>::new(self.data).has_any(tags)
    }

    /// Clears all flags.
    pub fn clear_tags(&mut self) {
        self.data = StatusFlagsData::zero();
    }
}

define_bitmask_flags!(Status, u64, {
    Stunned,
    Invulnerable,
});

// Whether the owner is in contact with the ground.
