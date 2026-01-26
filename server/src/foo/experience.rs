use shared::Owner;
use spacetimedb::{table, SpacetimeType};

/// The amount of experience this person has accumulated
#[table(name = experience_tbl)]
pub struct Experience {
    #[primary_key]
    pub owner: Owner,

    pub data: ExperienceData,
}
#[derive(SpacetimeType, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExperienceData {
    pub experience: u32,
}
crate::impl_data_table!(
    table_handle = experience_tbl,
    row = Experience,
    data = ExperienceData
);
