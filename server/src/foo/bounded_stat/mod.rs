pub mod health;
pub mod mana;

pub use health::*;
pub use mana::*;

use num_traits::PrimInt;

/// Trait for types representing a bounded numeric stat (health, mana, stamina, etc.).
///
/// Provides:
/// - Read access to current and max values
/// - Safe mutation with clamping (never exceeds max, never below 0)
/// - Saturating add/sub operations
///
///
/// Implementors must provide:
/// - `current()`
/// - `max()`
/// - `set_current(value)`
/// - `set_max(new_max)` (should clamp current ≤ new_max)
///
/// Default impls:
/// - `add(amount)`: increases current, caps at max
/// - `sub(amount)`: decreases current, floors at 0
pub trait BoundedStat<T: PrimInt> {
    fn current(&self) -> T;
    fn max(&self) -> T;

    fn set_current(&mut self, value: T);
    fn set_max(&mut self, new_max: T);

    fn add(&mut self, amount: T) {
        self.set_current(self.current().saturating_add(amount).min(self.max()));
    }
    fn sub(&mut self, amount: T) {
        self.set_current(self.current().saturating_sub(amount));
    }
}

/// Macro to generate a bounded stat type (e.g. HealthData, ManaData).
///
/// Generates:
/// - A struct named `<$name>Data` with exactly two `pub` fields:
///   - `current: $ty`
///   - `max: $ty`
/// - `#[derive(SpacetimeType, Debug, PartialEq, Eq, Clone, Copy)]`
/// - `impl BoundedStat<$ty>` with full getter/setter/clamp logic
/// - `impl <$name>Data { pub fn new(max: $ty) -> Self { … } }`
///
/// Intent: Provide consistent, zero-boilerplate bounded numeric stats
/// (health, mana, stamina, etc.) that auto-clamp on changes and integrate
/// with SpacetimeDB tables.
///
/// Example:
/// ```rust
/// bounded_stat!(Health, u16);
/// // expands to struct HealthData { pub current: u16, pub max: u16 }
/// let h = HealthData::new(100);
/// h.add(20);           // current = 100 (clamped)
/// h.sub(150);          // current = 0
/// h.set_max(80);       // current clamped to ≤80 if needed
/// ```
///
/// Usage: Call once per stat type in the appropriate module.
#[macro_export]
macro_rules! bounded_stat {
    ($name:ident, $ty:ty) => {
        use crate::foo::BoundedStat;
        use spacetimedb::SpacetimeType;

        #[derive(SpacetimeType, Debug, PartialEq, Eq, Clone, Copy)]
        pub struct $name {
            pub current: $ty,
            pub max: $ty,
        }

        impl BoundedStat<$ty> for $name {
            fn current(&self) -> $ty {
                self.current
            }
            fn max(&self) -> $ty {
                self.max
            }
            fn set_current(&mut self, value: $ty) {
                self.current = value;
            }
            fn set_max(&mut self, new_max: $ty) {
                self.max = new_max;
                if self.current > new_max {
                    self.current = new_max;
                }
            }
        }

        impl $name {
            pub fn new(max: $ty) -> Self {
                Self { current: max, max }
            }
        }
    };
}
