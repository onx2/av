use spacetimedb::*;

#[derive(Default, Debug)]
#[table(name = vital_stats)]
pub struct VitalStats {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    pub health: u16,
    pub mana: u16,
    pub stamina: u16,
}
