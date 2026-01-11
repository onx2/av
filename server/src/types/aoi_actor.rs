#[derive(spacetimedb::SpacetimeType, Debug)]
pub struct AoiActor {
    pub id: u64,
    pub transform_data_id: u64,
    pub secondary_stats_id: u32,
    pub identity: Option<spacetimedb::Identity>,
    pub grounded: bool,
    pub capsule_radius: f32,
    pub capsule_half_height: f32,
    pub move_intent: super::MoveIntent,
}
impl From<crate::schema::Actor> for AoiActor {
    fn from(actor: crate::schema::Actor) -> Self {
        Self {
            id: actor.id,
            transform_data_id: actor.transform_data_id,
            secondary_stats_id: actor.secondary_stats_id,
            identity: actor.identity,
            grounded: actor.grounded,
            capsule_radius: actor.capsule_radius,
            capsule_half_height: actor.capsule_half_height,
            move_intent: actor.move_intent,
        }
    }
}
