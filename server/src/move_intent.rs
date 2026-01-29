use super::Vec2;
use crate::transform_tbl__view;
use shared::Owner;
use spacetimedb::*;
use std::collections::HashMap;

// Ephemeral movement requests, existence filtering can be used in movement tick
// We iterate over this table and attempt to move any Owner for the move intent, removing
// the row when we've reached the end of the intent.
#[table(name=move_intent_tbl)]
pub struct MoveIntent {
    #[primary_key]
    pub owner: Owner,

    pub data: MoveIntentData,
}
impl MoveIntent {
    pub fn upsert(ctx: &spacetimedb::ReducerContext, owner: Owner, data: MoveIntentData) -> Self {
        // If the row doesn't exist, delete will return false, which we ignore.
        let _ = ctx.db.move_intent_tbl().owner().delete(owner);
        ctx.db.move_intent_tbl().insert(Self { owner, data })
    }
    pub fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self> {
        ctx.db.move_intent_tbl().owner().find(owner)
    }
    pub fn delete(&self, ctx: &ReducerContext) {
        ctx.db.move_intent_tbl().owner().delete(self.owner);
    }
    pub fn insert(ctx: &ReducerContext, owner: Owner, data: MoveIntentData) {
        ctx.db.move_intent_tbl().insert(Self { owner, data });
    }
}
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
