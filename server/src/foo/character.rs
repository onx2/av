use shared::owner::OwnerId;
use spacetimedb::{table, Identity};

use super::{PrimaryStatsData, SecondaryStatsData, TransformData, VitalStatsData};

/// The persistence layer for player character's
///
/// **Possible source of `owner` found in other tables.**
#[table(name=character_tbl)]
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
