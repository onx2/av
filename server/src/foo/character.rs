use shared::owner::{pack_owner, AsOwner, Owner, OwnerId, OwnerKind};
use spacetimedb::{table, Identity, ReducerContext, Table};

use super::{
    actor_tbl, health_tbl, mana_tbl, primary_stats_tbl, secondary_stats_tbl, transform_tbl, Actor,
    Health, HealthData, Mana, ManaData, PrimaryStats, PrimaryStatsData, SecondaryStats,
    SecondaryStatsData, Transform, TransformData,
};

/// The persistence layer for a player's characters
#[table(name=character_tbl)]
pub struct Character {
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    #[index(btree)]
    pub identity: Identity,

    #[unique]
    pub name: String,

    pub transform: TransformData,
    pub primary_stats: PrimaryStatsData,
    pub secondary_stats: SecondaryStatsData,
    pub health: HealthData,
    pub mana: ManaData,
}

impl AsOwner for Character {
    fn owner(&self) -> Owner {
        pack_owner(self.owner_id, OwnerKind::Character)
    }
    fn owner_id(&self) -> OwnerId {
        self.owner_id
    }
    fn owner_kind(&self) -> OwnerKind {
        OwnerKind::Character
    }
}

impl Character {
    pub fn create(
        ctx: &ReducerContext,
        name: impl Into<String>,
        primary_stats: PrimaryStatsData,
    ) -> Result<Owner, &'static str> {
        let name = name.into();
        let length = name.chars().count();
        if length < 3 || length > 64 {
            return Err("Name must be 3–64 characters");
        }
        if !name.chars().all(|c| c.is_alphanumeric()) {
            return Err("Name must be alphanumeric");
        }

        let inserted = ctx.db.character_tbl().insert(Character {
            owner_id: 0,
            identity: ctx.sender,
            name,
            transform: TransformData::default(),
            primary_stats,
            // TODO: build these other stats based on the primary_stats... but this required defining relationships
            secondary_stats: SecondaryStatsData::default(),
            health: HealthData::new(100),
            mana: ManaData::new(100),
        });

        Ok(pack_owner(inserted.owner_id, OwnerKind::Character))
    }

    pub fn leave_game(&self, ctx: &ReducerContext) {
        let owner = self.owner();
        ctx.db.actor_tbl().owner().delete(owner);
        ctx.db.transform_tbl().owner().delete(owner);
        ctx.db.primary_stats_tbl().owner().delete(owner);
        ctx.db.secondary_stats_tbl().owner().delete(owner);
        ctx.db.health_tbl().owner().delete(owner);
        ctx.db.mana_tbl().owner().delete(owner);
    }
    pub fn enter_game(&self, ctx: &ReducerContext) {
        // Prevent multiple player characters from joining the game, only one character per player
        for character in ctx.db.character_tbl().identity().filter(ctx.sender) {
            let owner = character.owner();
            ctx.db.actor_tbl().owner().delete(owner);
            ctx.db.transform_tbl().owner().delete(owner);
            ctx.db.primary_stats_tbl().owner().delete(owner);
            ctx.db.secondary_stats_tbl().owner().delete(owner);
            ctx.db.health_tbl().owner().delete(owner);
            ctx.db.mana_tbl().owner().delete(owner);
        }

        let owner = self.owner();
        ctx.db.actor_tbl().insert(Actor { owner });
        ctx.db.transform_tbl().insert(Transform {
            owner,
            data: self.transform,
        });
        ctx.db.primary_stats_tbl().insert(PrimaryStats {
            owner,
            data: self.primary_stats,
        });
        ctx.db.secondary_stats_tbl().insert(SecondaryStats {
            owner,
            data: self.secondary_stats,
        });
        ctx.db.health_tbl().insert(Health {
            owner,
            data: self.health,
        });
        ctx.db.mana_tbl().insert(Mana {
            owner,
            data: self.mana,
        });
    }
}
