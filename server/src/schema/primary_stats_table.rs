use spacetimedb::*;

#[derive(Default, Debug)]
#[table(name = primary_stats)]
pub struct PrimaryStats {
    #[primary_key]
    #[auto_inc]
    pub id: u32,

    pub strength: u8,
    pub dexterity: u8,
    pub fortitude: u8,
    pub intelligence: u8,
    pub piety: u8,
}
