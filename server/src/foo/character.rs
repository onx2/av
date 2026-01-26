use super::{
    active_character_tbl, experience_tbl, health_tbl, level_tbl, mana_tbl, primary_stats_tbl,
    transform_tbl, ActiveCharacter, DataTable, Experience, ExperienceData, Health, HealthData,
    Level, LevelData, Mana, ManaData, PrimaryStats, PrimaryStatsData, Transform, TransformData,
};
use shared::{pack_owner, AsOwner, Owner, OwnerId, OwnerKind};
use spacetimedb::{table, Identity, ReducerContext, Table};

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
    pub health: HealthData,
    pub mana: ManaData,
    pub experience: ExperienceData,
    pub level: LevelData,
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

        if !primary_stats.validate() {
            return Err("Primary stats are not valid");
        }

        let inserted = ctx.db.character_tbl().insert(Character {
            owner_id: 0,
            identity: ctx.sender,
            name,
            transform: TransformData::default(),
            primary_stats,
            experience: ExperienceData::default(),
            level: LevelData::default(),
            health: HealthData::new(100),
            mana: ManaData::new(100),
        });

        Ok(pack_owner(inserted.owner_id, OwnerKind::Character))
    }

    pub fn leave_game(&self, ctx: &ReducerContext) {
        let owner = self.owner();
        ctx.db.active_character_tbl().owner().delete(owner);
        ctx.db.transform_tbl().owner().delete(owner);
        ctx.db.primary_stats_tbl().owner().delete(owner);
        ctx.db.health_tbl().owner().delete(owner);
        ctx.db.mana_tbl().owner().delete(owner);
    }

    pub fn enter_game(&self, ctx: &ReducerContext) {
        // Prevent multiple player characters from joining the game, only one character per player
        for character in ctx.db.character_tbl().identity().filter(ctx.sender) {
            let owner = character.owner();
            ctx.db.active_character_tbl().owner().delete(owner);
            ctx.db.transform_tbl().owner().delete(owner);
            ctx.db.primary_stats_tbl().owner().delete(owner);
            ctx.db.health_tbl().owner().delete(owner);
            ctx.db.mana_tbl().owner().delete(owner);
            ctx.db.experience_tbl().owner().delete(owner);
            ctx.db.level_tbl().owner().delete(owner);
        }

        let owner = self.owner();
        ctx.db
            .active_character_tbl()
            .insert(ActiveCharacter::new(ctx.sender, owner));
        Transform::insert(ctx, owner, self.transform);
        PrimaryStats::insert(ctx, owner, self.primary_stats);
        Health::insert(ctx, owner, self.health);
        Mana::insert(ctx, owner, self.mana);
        Experience::insert(ctx, owner, self.experience);
        Level::insert(ctx, owner, self.level);
    }
}
