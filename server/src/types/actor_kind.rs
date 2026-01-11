/// Logical kind/ownership for an actor.
///
/// Extend as needed for NPCs, bosses, and other categories.
#[derive(spacetimedb::SpacetimeType, Debug, Clone, PartialEq)]
pub enum ActorKind {
    /// A player-controlled actor keyed by the user's identity.
    Player(spacetimedb::Identity),
    /// A simple monster/NPC variant.
    Monster(u32),
}
