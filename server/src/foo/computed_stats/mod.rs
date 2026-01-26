mod critical_hit_chance;
mod movement_speed;

use shared::Owner;
use spacetimedb::{LocalReadOnly, SpacetimeType};
use std::ops::Deref;

pub use critical_hit_chance::*;
pub use movement_speed::*;

/// A small, reusable spacetime payload for "scalar" stats.
#[derive(SpacetimeType)]
pub struct Stat<T: Copy + Default> {
    pub value: T,
}
impl<T: Copy + Default> Stat<T> {
    pub fn new(value: T) -> Self {
        Stat { value }
    }
}
impl<T: Copy + Default> Deref for Stat<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub trait ComputedStat {
    type Output: SpacetimeType;
    fn compute(db: &LocalReadOnly, owner: Owner) -> Option<Self::Output>;
}
