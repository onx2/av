pub mod constants;
/// Shared, deterministic logic intended to run identically on server and client.
///
/// Rapier transition
/// -----------------
/// This crate is transitioning to a Rapier-first architecture:
/// - The authoritative/static collision world is defined by DB rows.
/// - Server and client build the same in-memory Rapier query world from those rows.
/// - Movement/scene queries operate at the Rapier level (not Parry-level wrappers).
///
/// Public API policy
/// -----------------
/// Keep the public surface small and stable:
/// - `rapier_world`: schema-agnostic world definitions + builder for an in-memory Rapier query world.
///
/// Everything else should be considered internal and subject to change.
pub mod rapier_world;
pub mod utils;

// Re-exports for callers building the world from DB rows.
pub use rapier_world::{ColliderShapeDef, RapierQueryWorld, WorldStaticDef};
