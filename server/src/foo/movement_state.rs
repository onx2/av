use crate::impl_data_table;
use shared::Owner;
use spacetimedb::{table, SpacetimeType};

/// Ephemeral/computed & cached state for the owner's movement. This doesn't need to be persisted
/// and can be removed when the owner is removed from the world.
#[table(name=movement_state_tbl)]
pub struct MovementState {
    #[primary_key]
    pub owner: Owner,

    pub data: MovementStateData,
}
impl_data_table!(
    table_handle = movement_state_tbl,
    row = MovementState,
    data = MovementStateData
);

#[derive(SpacetimeType, Debug, Default, Copy, Clone)]
pub struct MovementStateData {
    /// Is the owner in contact with a surface, I.E. not "falling"
    pub grounded: bool,

    /// Tracked for gravity acceleration
    pub vertical_velocity: f32,
}
