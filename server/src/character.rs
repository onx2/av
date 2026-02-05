use crate::{
    actor_tbl, character_instance_tbl, experience_tbl, health_tbl, level_tbl, mana_tbl,
    movement_state_tbl, primary_stats_tbl, transform_tbl, ActorRow, CapsuleY, CharacterInstanceRow,
    ExperienceRow, HealthData, HealthRow, LevelRow, ManaData, ManaRow, MoveIntentData,
    MovementStateRow, PrimaryStatsRow, SecondaryStatsRow, TransformRow, Vec3,
};
use shared::{encode_cell_id, CellId};
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

    pub translation: Vec3,
    pub yaw: f32,

    // Primary stats
    pub ferocity: u8,
    pub fortitude: u8,
    pub intellect: u8,
    pub acuity: u8,
    pub available_points: u8,

    // Secondary stats are computed

    // Vitals
    pub health: HealthData,
    pub mana: ManaData,

    // Progression
    pub experience: u32,
    pub level: u8,
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

        let level = 1;
        let ferocity = PrimaryStatsRow::MIN_STAT;
        let fortitude = PrimaryStatsRow::MIN_STAT;
        let intellect = PrimaryStatsRow::MIN_STAT;
        let acuity = PrimaryStatsRow::MIN_STAT;
        let inserted = ctx.db.character_tbl().insert(CharacterRow {
            id: 0,
            identity: ctx.sender,
            name,
            yaw: 0.,
            translation: Vec3::new(0., 50.0, 0.),
            deleted: false,
            capsule: CapsuleY {
                radius: 0.3,
                half_height: 0.9,
            },

            ferocity,
            fortitude,
            intellect,
            acuity,
            available_points: 0,

            health: HealthData::new(HealthData::compute_max(level, fortitude)),
            mana: ManaData::new(ManaData::compute_max(level, intellect)),

            experience: 0,
            level,
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

        let cell_id: CellId = encode_cell_id(self.translation.x, self.translation.z);
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
            move_intent: MoveIntentData::None,
            vertical_velocity: -1,
            cell_id,
        });
        TransformRow::insert(ctx, actor.id, self.translation, self.yaw);
        PrimaryStatsRow::insert(
            ctx,
            actor.id,
            self.ferocity,
            self.fortitude,
            self.intellect,
            self.acuity,
            self.available_points,
        );
        let movement_speed = SecondaryStatsRow::compute_movement_speed(self.level, 0.0, 0.0, 0.0);
        let critical_hit_chance =
            SecondaryStatsRow::compute_critical_hit_chance(self.level, self.ferocity, 0.0);
        SecondaryStatsRow::insert(ctx, actor.id, movement_speed, critical_hit_chance);
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
