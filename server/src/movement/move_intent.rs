use crate::{transform_tbl__view, Vec2};
use rapier3d::parry::utils::hashmap::HashMap;
use shared::Owner;
use spacetimedb::*;

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

    /// Gets the next target position for the given MoveIntent, preferring the
    /// cached position of the target actor when possible. Avoid additional index seeks, when
    /// there is an actor multiple others are trying to follow.
    pub fn target_position_with_cache(
        &self,
        db: &LocalReadOnly,
        cache: &mut HashMap<Owner, Vec2>,
    ) -> Option<Vec2> {
        match &self {
            MoveIntentData::Point(point) => Some(*point),
            MoveIntentData::Path(path) => path.first().copied(),
            MoveIntentData::Actor(owner) => match cache.get(owner) {
                Some(pos) => Some(*pos),
                None => db.transform_tbl().owner().find(owner).map(|t| {
                    let xz = t.data.translation.xz();
                    cache.insert(*owner, xz);
                    xz
                }),
            },
        }
    }
}
