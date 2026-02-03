use crate::CapsuleY;
use shared::ActorId;
use spacetimedb::table;

/// Shared table for all instances
#[table(name=actor_tbl)]
pub struct ActorRow {
    #[auto_inc]
    #[primary_key]
    pub id: ActorId,

    /// 8 bytes right now but could be quantized to 4bytes
    pub capsule: CapsuleY,
}
