use num_traits::{One, PrimInt};

/// The default storage type for a packed set of flags.
pub type FlagsContainer = u64;

/// Implemented by enums that represent individual flags inside a bitmask.
///
/// A flag enum maps each variant to a bit index. By default (when using
/// `define_bitmask_flags!`), the bit index is the enum discriminant.
///
/// `mask()` converts a flag into its bitmask form: `1 << bit_index`.
pub trait FlagBitmask {
    /// The integer type the flag masks operate on (e.g. `u8`, `u16`, `u32`, `u64`).
    type Storage: PrimInt;

    /// The bit position for this flag (0-based).
    fn bit_index(&self) -> u8;

    /// The mask for this flag in `Storage`.
    fn mask(&self) -> Self::Storage {
        // NOTE: Ensure your `bit_index()` fits within the number of bits in `Storage`.
        Self::Storage::one() << (self.bit_index() as usize)
    }
}

/// A small, pure-Rust helper for working with packed bitmask flags.
///
/// This type is intentionally *not* tied to any database/replication system. Persist or replicate
/// the raw integer (`bits`) and use this wrapper for ergonomic operations in both client and server
/// code.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct BitmaskFlags<T: PrimInt> {
    /// The raw packed bitmask.
    pub bits: T,
}

impl<T: PrimInt> BitmaskFlags<T> {
    #[inline]
    fn combine_mask<U: FlagBitmask<Storage = T>>(tags: &[U]) -> T {
        tags.iter().fold(T::zero(), |acc, t| acc | t.mask())
    }

    /// Creates a new container from raw bits.
    pub fn new(bits: T) -> Self {
        Self { bits }
    }

    /// Sets a flag bit.
    pub fn add<U: FlagBitmask<Storage = T>>(&mut self, tag: U) {
        self.bits = self.bits | tag.mask();
    }

    /// Clears a flag bit.
    pub fn remove<U: FlagBitmask<Storage = T>>(&mut self, tag: U) {
        self.bits = self.bits & !tag.mask();
    }

    /// Returns true if the flag bit is set.
    pub fn has<U: FlagBitmask<Storage = T>>(&self, tag: U) -> bool {
        (self.bits & tag.mask()) != T::zero()
    }

    /// Sets all flags in `tags`.
    pub fn add_many<U: FlagBitmask<Storage = T>>(&mut self, tags: &[U]) {
        self.bits = self.bits | Self::combine_mask(tags);
    }

    /// Clears all flags in `tags`.
    pub fn remove_many<U: FlagBitmask<Storage = T>>(&mut self, tags: &[U]) {
        self.bits = self.bits & !Self::combine_mask(tags);
    }

    /// Returns true if *all* flags in `tags` are set.
    ///
    /// For an empty slice, this returns `true`.
    pub fn has_all<U: FlagBitmask<Storage = T>>(&self, tags: &[U]) -> bool {
        let m = Self::combine_mask(tags);
        (self.bits & m) == m
    }

    /// Returns true if *any* flag in `tags` is set.
    ///
    /// For an empty slice, this returns `false`.
    pub fn has_any<U: FlagBitmask<Storage = T>>(&self, tags: &[U]) -> bool {
        let m = Self::combine_mask(tags);
        (self.bits & m) != T::zero()
    }

    /// Clears all flags (sets bits to zero).
    pub fn clear(&mut self) {
        self.bits = T::zero();
    }
}

/// Defines a flag enum that can be stored inside a bitmask.
///
/// You specify:
/// - the enum name (`$name`)
/// - the integer storage type the flags will live in (`$storage`)
/// - the ordered list of variants
///
/// Each variant maps to a bit index equal to its enum discriminant, and the generated
/// `FlagBitmask` impl turns that into a mask (`1 << index`).
///
/// ## Breaking change warning (variant order matters)
/// The order of the variants *is part of your storage format*. If you persist/replicate the
/// underlying bitmask, then **reordering existing variants will reinterpret old data**.
///
/// Safe ways to evolve a flag enum:
/// - append new variants at the end, or
/// - assign explicit discriminants (`A = 0, B = 1, ...`) and never change them.
///
/// ## Example
/// ```rust
/// use shared::define_bitmask_flags;
///
/// define_bitmask_flags!(UnitStatus, u16, {
///     IsFriendly,
///     InCombat,
///     Stunned,
///     Slowed,
///     Invulnerable,
/// });
/// ```
#[macro_export]
macro_rules! define_bitmask_flags {
    ($name:ident, $storage:ty, { $($variant:ident),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum $name {
            $($variant),*
        }

        impl $crate::bitmask_flags::FlagBitmask for $name {
            type Storage = $storage;

            fn bit_index(&self) -> u8 {
                *self as u8
            }
        }
    };
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::define_bitmask_flags;

    define_bitmask_flags!(TestFlagsU8, u8, {
        A,
        B,
        C,
        D,
        E,
        F,
        G,
        H,
    });

    #[test]
    fn mask_is_one_shift_bit_index() {
        assert_eq!(TestFlagsU8::A.mask(), 1u8 << 0);
        assert_eq!(TestFlagsU8::B.mask(), 1u8 << 1);
        assert_eq!(TestFlagsU8::H.mask(), 1u8 << 7);
    }

    #[test]
    fn default_is_zero_and_has_is_false() {
        let flags = BitmaskFlags::<u8>::default();
        assert_eq!(flags.bits, 0);
        assert!(!flags.has(TestFlagsU8::A));
        assert!(!flags.has(TestFlagsU8::H));
    }

    #[test]
    fn add_sets_bit_and_has_returns_true() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        flags.add(TestFlagsU8::C);
        assert!(flags.has(TestFlagsU8::C));
        assert_eq!(flags.bits, TestFlagsU8::C.mask());

        // Adding again should be idempotent.
        flags.add(TestFlagsU8::C);
        assert!(flags.has(TestFlagsU8::C));
        assert_eq!(flags.bits, TestFlagsU8::C.mask());
    }

    #[test]
    fn add_multiple_tags_sets_union_of_bits() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        flags.add(TestFlagsU8::A);
        flags.add(TestFlagsU8::D);
        flags.add(TestFlagsU8::H);

        let expected = TestFlagsU8::A.mask() | TestFlagsU8::D.mask() | TestFlagsU8::H.mask();
        assert_eq!(flags.bits, expected);

        assert!(flags.has(TestFlagsU8::A));
        assert!(flags.has(TestFlagsU8::D));
        assert!(flags.has(TestFlagsU8::H));
        assert!(!flags.has(TestFlagsU8::B));
    }

    #[test]
    fn remove_clears_bit_and_does_not_affect_others() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        flags.add(TestFlagsU8::A);
        flags.add(TestFlagsU8::B);
        flags.add(TestFlagsU8::H);

        flags.remove(TestFlagsU8::B);

        assert!(flags.has(TestFlagsU8::A));
        assert!(!flags.has(TestFlagsU8::B));
        assert!(flags.has(TestFlagsU8::H));

        let expected = TestFlagsU8::A.mask() | TestFlagsU8::H.mask();
        assert_eq!(flags.bits, expected);
    }

    #[test]
    fn remove_is_idempotent() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        flags.add(TestFlagsU8::D);

        flags.remove(TestFlagsU8::A); // removing unset bit should do nothing
        assert_eq!(flags.bits, TestFlagsU8::D.mask());

        flags.remove(TestFlagsU8::D);
        assert_eq!(flags.bits, 0);

        flags.remove(TestFlagsU8::D); // removing again remains 0
        assert_eq!(flags.bits, 0);
    }

    #[test]
    fn add_many_adds_all() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        let tags = [TestFlagsU8::A, TestFlagsU8::C, TestFlagsU8::H];
        flags.add_many(&tags);

        assert!(flags.has(TestFlagsU8::A));
        assert!(flags.has(TestFlagsU8::C));
        assert!(flags.has(TestFlagsU8::H));

        let expected = TestFlagsU8::A.mask() | TestFlagsU8::C.mask() | TestFlagsU8::H.mask();
        assert_eq!(flags.bits, expected);
    }

    #[test]
    fn remove_many_removes_all() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        let tags = [TestFlagsU8::A, TestFlagsU8::C, TestFlagsU8::H];
        flags.add_many(&tags);

        flags.remove_many(&[TestFlagsU8::C, TestFlagsU8::A]);

        assert!(!flags.has(TestFlagsU8::A));
        assert!(!flags.has(TestFlagsU8::C));
        assert!(flags.has(TestFlagsU8::H));

        let expected = TestFlagsU8::H.mask();
        assert_eq!(flags.bits, expected);
    }

    #[test]
    fn has_all_empty_is_true() {
        let flags = BitmaskFlags::<u8>::new(0);
        let empty: [TestFlagsU8; 0] = [];
        assert!(flags.has_all(&empty));
    }

    #[test]
    fn has_any_empty_is_false() {
        let flags = BitmaskFlags::<u8>::new(0);
        let empty: [TestFlagsU8; 0] = [];
        assert!(!flags.has_any(&empty));
    }

    #[test]
    fn has_all_and_has_any_semantics() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        flags.add(TestFlagsU8::A);
        flags.add(TestFlagsU8::D);

        assert!(flags.has_all(&[TestFlagsU8::A]));
        assert!(flags.has_all(&[TestFlagsU8::A, TestFlagsU8::D]));
        assert!(!flags.has_all(&[TestFlagsU8::A, TestFlagsU8::B]));

        assert!(flags.has_any(&[TestFlagsU8::B, TestFlagsU8::A]));
        assert!(flags.has_any(&[TestFlagsU8::D]));
        assert!(!flags.has_any(&[TestFlagsU8::B, TestFlagsU8::C]));
    }

    #[test]
    fn clear_sets_bits_to_zero() {
        let mut flags = BitmaskFlags::<u8>::new(0);
        flags.add(TestFlagsU8::A);
        flags.add(TestFlagsU8::H);
        assert_ne!(flags.bits, 0);

        flags.clear();
        assert_eq!(flags.bits, 0);
        assert!(!flags.has(TestFlagsU8::A));
        assert!(!flags.has(TestFlagsU8::H));
    }

    #[test]
    fn can_start_from_nonzero_bits_and_mutate_correctly() {
        // Start with A and H set.
        let initial = TestFlagsU8::A.mask() | TestFlagsU8::H.mask();
        let mut flags = BitmaskFlags::<u8>::new(initial);

        assert!(flags.has(TestFlagsU8::A));
        assert!(flags.has(TestFlagsU8::H));
        assert!(!flags.has(TestFlagsU8::B));

        // Add B and remove H.
        flags.add(TestFlagsU8::B);
        flags.remove(TestFlagsU8::H);

        assert!(flags.has(TestFlagsU8::A));
        assert!(flags.has(TestFlagsU8::B));
        assert!(!flags.has(TestFlagsU8::H));

        let expected = TestFlagsU8::A.mask() | TestFlagsU8::B.mask();
        assert_eq!(flags.bits, expected);
    }
}
