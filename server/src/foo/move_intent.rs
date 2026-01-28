use crate::{foo::transform_tbl__view, impl_data_table};

use super::Vec2;
use shared::Owner;
use spacetimedb::*;

// Ephemeral movement requests, existence filtering can be used in movement tick
// We iterate over this table and attempt to move any Owner for the move intent, removing
// the row when we've reached the end of the intent.
#[table(name=move_intent_tbl)]
pub struct MoveIntent {
    #[primary_key]
    pub owner: Owner,

    pub data: MoveIntentData,
}
impl_data_table!(
    table_handle = move_intent_tbl,
    row = MoveIntent,
    data = MoveIntentData
);
/// Represents the 2-dimensional movement intent of an Actor in the world
#[derive(SpacetimeType, Debug, Clone, PartialEq)]
pub enum MoveIntentData {
    /// Movement toward a specific position in the world
    Point(Vec2),
    /// Movement along a path
    Path(Vec<Vec2>),
    /// Movement toward an entity in the world (Actor)
    Actor(Owner),
}
impl MoveIntentData {
    /// Gets the next target position for the given MoveIntent
    pub fn target_position(&self, db: &LocalReadOnly) -> Option<Vec2> {
        match &self {
            MoveIntentData::Point(point) => Some(*point),
            MoveIntentData::Path(path) => path.first().copied(),
            MoveIntentData::Actor(owner) => db
                .transform_tbl()
                .owner()
                .find(owner)
                .map(|t| t.data.translation.xz()),
        }
    }
}
