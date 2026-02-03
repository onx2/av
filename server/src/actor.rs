use crate::CapsuleY;
use spacetimedb::table;

/// 8byte unique identifier for an actor.
pub type ActorId = u64;

/// Used to identify the definition table for this actor, character or monster.
// #[derive(SpacetimeType, Debug)]
// pub enum ActorKind {
//     Monster(u16),
//     Character(u32),
// }

/// Shared table for all instances
#[table(name=actor_tbl)]
pub struct ActorRow {
    #[auto_inc]
    #[primary_key]
    pub id: ActorId,
    // Between 2-4bytes, TODO do I need this?
    // pub kind: ActorKind,
    /// 8 bytes right now but could be quantized to 4bytes
    pub capsule: CapsuleY,
}
