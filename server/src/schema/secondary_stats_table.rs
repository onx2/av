use spacetimedb::*;

#[derive(Default, Debug)]
#[table(name = secondary_stats)]
pub struct SecondaryStats {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    /// Nominal horizontal movement speed (m/s), computed value
    pub movement_speed: f32,

    pub max_health: u16,
    pub max_mana: u16,
    pub max_stamina: u16,
}
