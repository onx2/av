#[derive(spacetimedb::SpacetimeType, Debug)]
pub struct AoiSecondaryStats {
    pub id: u32,
    pub actor_id: u64,
    pub transform_data_id: u64,
    pub movement_speed: f32,
    pub max_health: u16,
    pub max_mana: u16,
    pub max_stamina: u16,
}
