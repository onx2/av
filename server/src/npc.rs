use spacetimedb::table;

/// The persistence layer for the types of enemies that can be spawned into the world (Actor)
///
/// **Possible source of `owner` found in other tables.**
#[table(name=npc_tbl)]
pub struct NpcRow {
    #[auto_inc]
    #[primary_key]
    pub id: u16,

    pub name: String,
}
