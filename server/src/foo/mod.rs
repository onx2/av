pub mod actor;
pub mod character;
pub mod monster;
pub mod move_intent;
pub mod npc;
pub mod player;
pub mod primary_stats;
pub mod quat;
pub mod secondary_stats;
pub mod transform;
pub mod vector;
pub mod vital_stats;

pub use actor::{actor_tbl, Actor};
pub use character::{character_tbl, Character};
pub use monster::{monster_tbl, Monster};
pub use npc::{npc_tbl, Npc};
pub use player::{player_tbl, Player};
pub use primary_stats::{primary_stats_tbl, PrimaryStats, PrimaryStatsData};
pub use quat::Quat;
pub use secondary_stats::{secondary_stats_tbl, SecondaryStats, SecondaryStatsData};
pub use transform::{transform_tbl, Transform, TransformData};
pub use vector::{Vec2, Vec3};
pub use vital_stats::{vital_stats_tbl, VitalStats, VitalStatsData};

use shared::owner::{pack_owner, Owner, OwnerId, OwnerKind};
use spacetimedb::{ReducerContext, Table};

pub trait DataOwner {
    fn owner(&self) -> Owner;
    fn owner_id(&self) -> OwnerId;
    fn owner_kind(&self) -> OwnerKind;

    fn transform(&self) -> &TransformData;
    fn primary_stats(&self) -> &PrimaryStatsData;
    fn secondary_stats(&self) -> &SecondaryStatsData;
    fn vital_stats(&self) -> &VitalStatsData;

    fn upsert_ephemeral_data(&self, ctx: &ReducerContext) {
        let owner = self.owner();

        ctx.db.actor_tbl().owner().delete(owner);
        ctx.db.actor_tbl().insert(Actor { owner });

        ctx.db.transform_tbl().owner().delete(owner);
        ctx.db.transform_tbl().insert(Transform {
            owner,
            data: self.transform().clone(),
        });

        ctx.db.primary_stats_tbl().owner().delete(owner);
        ctx.db.primary_stats_tbl().insert(PrimaryStats {
            owner,
            data: self.primary_stats().clone(),
        });

        ctx.db.secondary_stats_tbl().owner().delete(owner);
        ctx.db.secondary_stats_tbl().insert(SecondaryStats {
            owner,
            data: self.secondary_stats().clone(),
        });

        ctx.db.vital_stats_tbl().owner().delete(owner);
        ctx.db.vital_stats_tbl().insert(VitalStats {
            owner,
            data: self.vital_stats().clone(),
        });
    }
}

impl DataOwner for Character {
    fn owner(&self) -> Owner {
        pack_owner(self.owner_id, OwnerKind::Character)
    }
    fn owner_id(&self) -> OwnerId {
        self.owner_id
    }
    fn owner_kind(&self) -> OwnerKind {
        OwnerKind::Character
    }
    fn transform(&self) -> &TransformData {
        &self.transform
    }
    fn primary_stats(&self) -> &PrimaryStatsData {
        &self.primary_stats
    }
    fn secondary_stats(&self) -> &SecondaryStatsData {
        &self.secondary_stats
    }
    fn vital_stats(&self) -> &VitalStatsData {
        &self.vital_stats
    }
}
