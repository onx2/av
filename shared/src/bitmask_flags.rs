use num_traits::{One, PrimInt};

/// The default primitive storage for bitmask flags.
/// You can still define flag enums backed by smaller sizes (u16/u32) when appropriate.
pub type FlagsContainer = u64;

/// Trait implemented by user-defined flag enums.
///
/// The enum's discriminant (via `#[repr(u8)]`) typically determines the bit index.
/// You choose the backing integer type via the associated `Storage`.
pub trait FlagBitmask {
    type Storage: PrimInt;

    fn bit_index(&self) -> u8;

    fn mask(&self) -> Self::Storage {
        // Equivalent to: 1 << index
        // NOTE: Ensure your `bit_index()` is < number of bits in `Storage`.
        Self::Storage::one() << (self.bit_index() as usize)
    }
}

/// A pure, shared bitmask container.
///
/// This is intentionally NOT a SpacetimeDB type. The server-side SpacetimeDB tables should store
/// primitives (e.g. `u64`) or server-only wrappers that derive `SpacetimeType`.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct BitmaskFlags<T: PrimInt> {
    pub bits: T,
}

impl<T: PrimInt> BitmaskFlags<T> {
    pub fn new(bits: T) -> Self {
        Self { bits }
    }

    // --- Single Tag Operations ---
    pub fn add<U: FlagBitmask<Storage = T>>(&mut self, tag: U) {
        self.bits = self.bits | tag.mask();
    }

    pub fn remove<U: FlagBitmask<Storage = T>>(&mut self, tag: U) {
        self.bits = self.bits & !tag.mask();
    }

    pub fn has<U: FlagBitmask<Storage = T>>(&self, tag: U) -> bool {
        (self.bits & tag.mask()) != T::zero()
    }

    // --- Bulk Operations ---
    pub fn add_many<U: FlagBitmask<Storage = T> + Copy>(&mut self, tags: &[U]) {
        for &tag in tags {
            self.add(tag);
        }
    }

    pub fn remove_many<U: FlagBitmask<Storage = T> + Copy>(&mut self, tags: &[U]) {
        for &tag in tags {
            self.remove(tag);
        }
    }

    // --- Logic Gates ---
    pub fn has_all<U: FlagBitmask<Storage = T> + Copy>(&self, tags: &[U]) -> bool {
        if tags.is_empty() {
            return true;
        }
        let combined = tags.iter().fold(T::zero(), |acc, t| acc | t.mask());
        (self.bits & combined) == combined
    }

    pub fn has_any<U: FlagBitmask<Storage = T> + Copy>(&self, tags: &[U]) -> bool {
        if tags.is_empty() {
            return false;
        }
        let combined = tags.iter().fold(T::zero(), |acc, t| acc | t.mask());
        (self.bits & combined) != T::zero()
    }

    pub fn clear(&mut self) {
        self.bits = T::zero();
    }
}

/// Declare a bitmask-backed enum and implement `FlagBitmask` for it.
///
/// Example:
/// ```rust
/// define_bitmask_flags!(UnitStatus, u16, {
///     IsFriendly,
///     InCombat,
///     Stunned,
///     Burning,
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
