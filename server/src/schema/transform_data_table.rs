use crate::types::*;
use spacetimedb::*;

#[derive(Default, Debug)]
#[table(name = transform_data)]
pub struct TransformData {
    #[primary_key]
    #[auto_inc]
    pub id: u64,

    pub translation: DbVec3,

    /// Quantized yaw (radians) stored as two bytes (`0..=~65k` maps onto `[0, 2π)`)
    pub yaw: u16,
}
