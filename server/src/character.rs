use crate::{
    actor_tbl, character_instance_tbl, experience_tbl, health_tbl, level_tbl, mana_tbl,
    movement_state_tbl, primary_stats_tbl, transform_tbl, ActorRow, CapsuleY, CharacterInstanceRow,
    ExperienceData, ExperienceRow, HealthData, HealthRow, LevelData, LevelRow, ManaData, ManaRow,
    MovementStateRow, PrimaryStatsData, PrimaryStatsRow, SecondaryStatsData, SecondaryStatsRow,
    TransformData, TransformRow, Vec3,
};
use shared::encode_cell_id;
use spacetimedb::{reducer, table, Identity, ReducerContext, Table};

/// The persistence layer for a player's characters
#[table(name=character_tbl)]
pub struct CharacterRow {
    #[auto_inc]
    #[primary_key]
    pub id: u32,

    #[index(btree)]
    pub identity: Identity,

    #[unique]
    pub name: String,

    #[index(btree)]
    pub deleted: bool,

    pub capsule: CapsuleY,

    pub transform: TransformData,
    pub primary_stats: PrimaryStatsData,
    pub secondary_stats: SecondaryStatsData,
    pub health: HealthData,
    pub mana: ManaData,
    pub experience: ExperienceData,
    pub level: LevelData,
}

impl CharacterRow {
    pub fn create(
        ctx: &ReducerContext,
        name: impl Into<String>,
    ) -> Result<CharacterRow, &'static str> {
        let name = name.into();
        let length = name.chars().count();
        if length < 3 || length > 64 {
            return Err("Name must be 3–64 characters");
        }
        if !name.chars().all(|c| c.is_alphanumeric()) {
            return Err("Name must be alphanumeric");
        }

        let level_data = LevelData::default();
        let primary_stats = PrimaryStatsData::default();
        let inserted = ctx.db.character_tbl().insert(CharacterRow {
            id: 0,
            identity: ctx.sender,
            name,
            transform: TransformData {
                yaw: 0,
                translation: Vec3::new(0., 50.0, 0.),
            },
            primary_stats,
            secondary_stats: SecondaryStatsData {
                movement_speed: SecondaryStatsData::compute_movement_speed(
                    level_data.level,
                    0.0,
                    0.0,
                    0.0,
                ),
                critical_hit_chance: SecondaryStatsData::compute_critical_hit_chance(
                    level_data.level,
                    primary_stats.ferocity,
                    0.0,
                ),
            },
            deleted: false,
            experience: ExperienceData::default(),
            level: level_data,
            health: HealthData::new(HealthData::compute_max(
                level_data.level,
                primary_stats.fortitude,
            )),
            mana: ManaData::new(ManaData::compute_max(
                level_data.level,
                primary_stats.intellect,
            )),
            capsule: CapsuleY {
                radius: 0.3,
                half_height: 0.9,
            },
        });

        Ok(inserted)
    }

    pub fn delete(&self, _ctx: &ReducerContext) -> bool {
        todo!("delete character todo")
    }

    fn delete_orphaned_rows(ctx: &ReducerContext) {
        let Some(ci) = ctx.db.character_instance_tbl().identity().find(&ctx.sender) else {
            log::error!("Unable to find actor for orphaned rows.");
            return;
        };

        ctx.db.transform_tbl().actor_id().delete(ci.actor_id);
        ctx.db.primary_stats_tbl().actor_id().delete(ci.actor_id);
        ctx.db.health_tbl().actor_id().delete(ci.actor_id);
        ctx.db.mana_tbl().actor_id().delete(ci.actor_id);
        ctx.db.experience_tbl().actor_id().delete(ci.actor_id);
        ctx.db.level_tbl().actor_id().delete(ci.actor_id);
        ctx.db.movement_state_tbl().actor_id().delete(ci.actor_id);
        ctx.db.actor_tbl().id().delete(ci.actor_id);
        ctx.db.character_instance_tbl().delete(ci);
    }

    pub fn leave_game(&self, ctx: &ReducerContext) {
        Self::delete_orphaned_rows(ctx);
    }

    pub fn enter_game(&self, ctx: &ReducerContext) {
        // Prevent multiple player characters from joining the game, only one character per player
        self.leave_game(ctx);

        let cell_id = encode_cell_id(self.transform.translation.x, self.transform.translation.z);
        let actor = ctx.db.actor_tbl().insert(ActorRow {
            id: 0,
            capsule: self.capsule,
        });
        ctx.db
            .character_instance_tbl()
            .insert(CharacterInstanceRow::new(ctx.sender, actor.id, self.id));
        ctx.db.movement_state_tbl().insert(MovementStateRow {
            actor_id: actor.id,
            should_move: true,
            move_intent: None,
            vertical_velocity: -1,
            cell_id,
        });
        TransformRow::insert(ctx, actor.id, self.transform);
        PrimaryStatsRow::insert(ctx, actor.id, self.primary_stats);
        SecondaryStatsRow::insert(ctx, actor.id, self.secondary_stats);
        HealthRow::insert(ctx, actor.id, self.health);
        ManaRow::insert(ctx, actor.id, self.mana);
        ExperienceRow::insert(ctx, actor.id, self.experience);
        LevelRow::insert(ctx, actor.id, self.level);
    }
}

#[reducer]
pub fn create_character(ctx: &ReducerContext, name: String) -> Result<(), String> {
    CharacterRow::create(ctx, name)
        .map(|_| ())
        .map_err(|e| e.into())
}

// TODO: make this correct again, this is changed to just find the first char for testing
#[reducer]
pub fn enter_game(ctx: &ReducerContext, character_id: u32) -> Result<(), String> {
    // let Some(character) = ctx.db.character_tbl().owner_id().find(character_id) else {
    //     return Err("Character not found".into());
    // };
    // if character.identity != ctx.sender {
    //     return Err("Unauthorized".into());
    // }

    let Ok(character) = CharacterRow::create(ctx, ctx.sender.to_string()) else {
        return Err("Failed to create character".into());
    };
    Ok(character.enter_game(ctx))
}

// #[reducer]
// pub fn delete_character(ctx: &ReducerContext, character_id: CharacterId) {
//     Character::delete(ctx, character_id).map(||());
// }
