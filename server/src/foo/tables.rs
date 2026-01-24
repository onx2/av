use super::{quat::Quat, vector::Vec3};
use shared::owner::{pack_owner, Owner, OwnerId, OwnerKind};
use spacetimedb::*;

/// Main persistence table a person's "account"
#[table(name=player)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
}

/// The persistence layer for player character's
///
/// **Possible source of `owner` found in other tables.**
#[table(name=character)]
pub struct Character {
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    #[index(btree)]
    pub identity: Identity,

    pub name: String,

    pub transform: TransformData,
    pub primary_stats: PrimaryStatsData,
    pub secondary_stats: SecondaryStatsData,
    pub vital_stats: VitalStatsData,
}

/// The persistence layer for the types of enemies that can be spawned into the world (Actor)
///
/// **Possible source of `owner` found in other tables.**
#[table(name=npc)]
pub struct Npc {
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    pub name: String,
}

/// The persistence layer for the types of enemies that can be spawned into the world (Actor)
///
/// **Possible source of `owner` found in other tables.**
#[table(name=monster)]
pub struct Monster {
    #[auto_inc]
    #[primary_key]
    pub owner_id: OwnerId,

    pub name: String,
}

/// Ephemeral
///
/// In-game, ephemeral, representation of a player's Character, a NPC, or a Monster.
///
/// Right now I'm thinking this is essentially a marker row for when something is spawned into the world with all its data.
/// TBD though on what this should hold... Might remove it entirely in favor of individual private tables and public views.
#[table(name=actor)]
pub struct Actor {
    #[primary_key]
    pub owner: Owner,
}

/// Ephemeral
///
/// The storage for various objects' transform data
#[table(name=transform)]
pub struct Transform {
    #[primary_key]
    pub owner: Owner,

    pub data: TransformData,
}
#[derive(SpacetimeType, Debug, PartialEq, Clone)]
pub struct TransformData {
    pub translation: Vec3,
    pub rotation: Quat,
}

/// Ephemeral
///
/// The primary driving factors for other aspects of gameplay (secondary stats, damage, etc...)
#[table(name=primary_stats)]
pub struct PrimaryStats {
    #[primary_key]
    pub owner: Owner,

    pub data: PrimaryStatsData,
}
#[derive(SpacetimeType, Debug, PartialEq, Clone)]
pub struct PrimaryStatsData {
    pub strength: u8,
    pub dexterity: u8,
    pub fortitude: u8,
    pub intelligence: u8,
    pub piety: u8,
}

///
///
/// The derived / computed stats for an owner, based on various things like PrimaryStats, equipment, perks, spells, etc...
#[table(name=secondary_stats)]
pub struct SecondaryStats {
    #[primary_key]
    pub owner: Owner,

    pub data: SecondaryStatsData,
}
#[derive(SpacetimeType, Debug, PartialEq, Clone)]
pub struct SecondaryStatsData {
    pub max_health: u16,
    pub max_mana: u16,
    pub max_stamina: u16,
    pub movement_speed: f32,
}

/// **Ephemeral**
///
/// The storage for Vitals data like "health". Intentionally isolated and slim because it is considered "hot", meaning it is
/// expected to change more frequently than other types of data.
#[table(name=vital_stats)]
pub struct VitalStats {
    #[primary_key]
    pub owner: Owner,

    pub data: VitalStatsData,
}
#[derive(SpacetimeType, Debug, PartialEq, Clone)]
pub struct VitalStatsData {
    pub health: u16,
    pub mana: u16,
    pub stamina: u16,
}

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

        ctx.db.actor().owner().delete(owner);
        ctx.db.actor().insert(Actor { owner });

        ctx.db.transform().owner().delete(owner);
        ctx.db.transform().insert(Transform {
            owner,
            data: self.transform().clone(),
        });

        ctx.db.primary_stats().owner().delete(owner);
        ctx.db.primary_stats().insert(PrimaryStats {
            owner,
            data: self.primary_stats().clone(),
        });

        ctx.db.secondary_stats().owner().delete(owner);
        ctx.db.secondary_stats().insert(SecondaryStats {
            owner,
            data: self.secondary_stats().clone(),
        });

        ctx.db.vital_stats().owner().delete(owner);
        ctx.db.vital_stats().insert(VitalStats {
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
