use super::{
    monster_instance_tbl, movement_state_tbl, CapsuleY, HealthData, HealthRow, ManaData, ManaRow,
    MonsterInstanceRow, MovementStateRow, PrimaryStatsData, PrimaryStatsRow, StatusFlags,
    StatusFlagsData, TransformData, TransformRow,
};
use shared::{encode_cell_id, pack_owner, Owner, OwnerKind};
use spacetimedb::{table, ReducerContext, Table};

/// Monster "definition" (type).
///
/// One row per monster kind/type you can spawn (e.g. Troll, Black Spider, Bug).
/// This is NOT a spawned world instance.
#[table(name=monster_tbl)]
pub struct MonsterRow {
    #[auto_inc]
    #[primary_key]
    pub id: u16,

    pub name: String,

    pub capsule: CapsuleY,
}

impl MonsterRow {
    pub fn insert(name: impl Into<String>, capsule: CapsuleY) -> Self {
        Self {
            id: 0,
            name: name.into(),
            capsule,
        }
    }

    /// Spawn a new monster instance (an [`Actor`]) from this monster definition.
    ///
    /// This allocates a fresh `owner_id` via `monster_instance_tbl` so multiple monsters of the
    /// same type can exist at once.
    // pub fn spawn_instance(&self, ctx: &ReducerContext) -> Result<Owner, String> {
    //     // Allocate a new instance id (owner_id) that will become the Actor/Owner key.
    //     let instance = ctx.db.monster_instance_tbl().insert(MonsterInstanceRow {
    //         owner_id: 0,
    //         monster_id: self.id,
    //     });

    //     let owner = pack_owner(instance.owner_id, OwnerKind::Monster);
    //     // Spawn at origin by default for now; call sites can update transform after spawn
    //     // (or you can extend this API to accept a transform).
    //     let transform: TransformData = Default::default();

    //     let cell_id = encode_cell_id(transform.translation.x, transform.translation.z);
    //     // Ephemeral component rows keyed by Owner.
    //     ctx.db.movement_state_tbl().insert(MovementStateRow {
    //         owner,
    //         grounded: false,
    //         should_move: true,
    //         move_intent: None,
    //         vertical_velocity: 0.0,
    //         cell_id,
    //         capsule: self.capsule,
    //     });
    //     TransformRow::insert(ctx, owner, transform);
    //     PrimaryStatsRow::insert(ctx, owner, PrimaryStatsData::default());
    //     HealthRow::insert(ctx, owner, HealthData::new(100));
    //     ManaRow::insert(ctx, owner, ManaData::new(100));
    //     StatusFlags::insert(ctx, owner, StatusFlagsData::default());

    //     Ok(owner)
    // }

    pub fn regenerate(ctx: &ReducerContext) {
        ctx.db.monster_tbl().iter().for_each(|row| {
            ctx.db.monster_tbl().delete(row);
        });

        MonsterRow::insert(
            "Troll",
            CapsuleY {
                radius: 0.3,
                half_height: 0.9,
            },
        );
    }
}
