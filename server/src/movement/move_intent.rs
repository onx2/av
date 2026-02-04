use crate::{transform_tbl__view, Vec2};
use rapier3d::parry::utils::hashmap::HashMap;
use shared::ActorId;
use spacetimedb::*;

/// Represents the 2-dimensional movement intent of an Actor in the world
#[derive(SpacetimeType, Debug, Clone, PartialEq)]
pub enum MoveIntentData {
    /// Movement toward a specific position in the world
    Point(Vec2),
    /// Movement along a path
    /// TODO: might be able to remove this in favor of point only and request
    /// a new point when it is reached... at least on the client? Not sure if that works on the server.
    Path(Vec<Vec2>),
    /// Movement toward an entity in the world (Actor)
    Actor(ActorId),
}
impl MoveIntentData {
    /// Gets the next target position for the given MoveIntent
    pub fn target_position(&self, db: &LocalReadOnly) -> Option<Vec2> {
        match &self {
            MoveIntentData::Point(point) => Some(*point),
            MoveIntentData::Path(path) => path.first().copied(),
            MoveIntentData::Actor(actor_id) => db
                .transform_tbl()
                .actor_id()
                .find(actor_id)
                .map(|t| t.data.translation.xz()),
        }
    }

    /// Gets the next target position for the given MoveIntent, preferring the
    /// cached position of the target actor when possible. Avoid additional index seeks, when
    /// there is an actor multiple others are trying to follow.
    pub fn target_position_with_cache(
        &self,
        db: &LocalReadOnly,
        cache: &mut HashMap<ActorId, Vec2>,
    ) -> Option<Vec2> {
        match &self {
            MoveIntentData::Point(point) => Some(*point),
            MoveIntentData::Path(path) => path.first().copied(),
            MoveIntentData::Actor(actor_id) => match cache.get(actor_id) {
                Some(pos) => Some(*pos),
                None => db.transform_tbl().actor_id().find(actor_id).map(|t| {
                    let xz = t.data.translation.xz();
                    cache.insert(*actor_id, xz);
                    xz
                }),
            },
        }
    }
}
