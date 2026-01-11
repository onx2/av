/// Movement intent for an actor.
///
/// Match arms are handled by the server's tick reducer; unsupported variants
/// can be extended in the future.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, PartialEq)]
pub enum MoveIntent {
    /// Follow a sequence of waypoints (in world space) across multiple frames.
    Path(Vec<super::DbVec3>),

    /// Follow a dynamic actor by id.
    Actor(u64),

    /// Move toward this point (direction) for a single frame.
    Point(super::DbVec3),

    /// No movement intent (idling).
    None,
}
