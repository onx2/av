use shared::{encode_cell_id, pack_owner, Owner, OwnerKind};
use spacetimedb::{table, ReducerContext, Table};

use crate::foo::{
    actor_tbl, monster_instance_tbl, Actor, DataTable, Experience, ExperienceData, Health,
    HealthData, Level, LevelData, Mana, ManaData, MonsterInstance, PrimaryStats, PrimaryStatsData,
    Transform, TransformData,
};

/// Monster "definition" (type).
///
/// One row per monster kind/type you can spawn (e.g. Troll, Black Spider, Bug).
/// This is NOT a spawned world instance.
#[table(name=monster_tbl)]
pub struct Monster {
    #[auto_inc]
    #[primary_key]
    pub monster_id: u16,

    pub name: String,
}

impl Monster {
    /// Spawn a new monster instance (an [`Actor`]) from this monster definition.
    ///
    /// This allocates a fresh `owner_id` via `monster_instance_tbl` so multiple monsters of the
    /// same type can exist at once.
    pub fn spawn_instance(&self, ctx: &ReducerContext) -> Result<Owner, String> {
        // Allocate a new instance id (owner_id) that will become the Actor/Owner key.
        let instance = ctx.db.monster_instance_tbl().insert(MonsterInstance {
            owner_id: 0,
            monster_id: self.monster_id,
        });

        let owner = pack_owner(instance.owner_id, OwnerKind::Monster);

        // Spawn at origin by default for now; call sites can update transform after spawn
        // (or you can extend this API to accept a transform).
        let transform: TransformData = Default::default();
        let cell_id = encode_cell_id(transform.translation.x, transform.translation.z);

        // Actor row marks this instance as "in the world".
        ctx.db.actor_tbl().insert(Actor { owner, cell_id });

        // Ephemeral component rows keyed by Owner.
        Transform::insert(ctx, owner, transform);
        PrimaryStats::insert(ctx, owner, PrimaryStatsData::default());
        Health::insert(ctx, owner, HealthData::new(100));
        Mana::insert(ctx, owner, ManaData::new(100));
        Experience::insert(ctx, owner, ExperienceData::default());
        Level::insert(ctx, owner, LevelData::default());

        Ok(owner)
    }
}
