use shared::ActorId;
use spacetimedb::table;

/// A spawned monster instance in the world.
#[table(name=monster_instance_tbl)]
pub struct MonsterInstanceRow {
    #[primary_key]
    pub actor_id: ActorId,

    /// Monster definition/type id from `monster_tbl`.
    #[index(btree)]
    pub monster_id: u16,
}
