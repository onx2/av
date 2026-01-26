use crate::foo::transform_tbl__view;

use super::Vec2;
use shared::Owner;
use spacetimedb::*;

#[table(name=move_intent_tbl)]
pub struct MoveIntent {
    #[primary_key]
    owner: Owner,

    data: MoveIntentData,
}
impl MoveIntent {
    /// Gets the next target position for the given MoveIntent
    pub fn target_position(&self, db: &LocalReadOnly) -> Option<Vec2> {
        match &self.data {
            MoveIntentData::None => None,
            MoveIntentData::Dir(dir) => Some(*dir),
            MoveIntentData::Path(path) => path.first().copied(),
            MoveIntentData::Actor(owner) => db
                .transform_tbl()
                .owner()
                .find(owner)
                .map(|t| t.data.translation.xz()),
        }
    }
}

/// Represents the 2-dimensional movement intent of an Actor in the world
#[derive(SpacetimeType, Debug, Clone, PartialEq)]
pub enum MoveIntentData {
    None,
    /// Movement toward a specific direction in the world
    Dir(Vec2),
    /// Movement along a path
    Path(Vec<Vec2>),
    /// Movement toward an entity in the world (Actor)
    Actor(Owner),
}
impl Default for MoveIntentData {
    fn default() -> Self {
        Self::None
    }
}
impl MoveIntentData {
    /// Mutably removes and returns the first point from the path (if any).
    pub fn consume_path_point(&mut self) -> Option<Vec2> {
        match self {
            MoveIntentData::Path(path) if !path.is_empty() => {
                let point = path.remove(0);
                Some(point)
            }
            _ => None,
        }
    }
}
