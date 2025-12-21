use crate::schema::{actor, actor_in_aoi, ActorInAoi};
use shared::utils::get_aoi_block;
use spacetimedb::*;

#[table(name = aoi_tick_timer, scheduled(aoi_tick_reducer))]
pub struct AoiTickTimer {
    /// Primary key for the scheduled job (single row used).
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    /// When/how often to invoke the scheduled reducer.
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn init(ctx: &ReducerContext) {
    let tick_interval = TimeDuration::from_micros(1_000_000 / 10);
    ctx.db.aoi_tick_timer().scheduled_id().delete(1);
    ctx.db.aoi_tick_timer().insert(AoiTickTimer {
        scheduled_id: 1,
        scheduled_at: spacetimedb::ScheduleAt::Interval(tick_interval),
    });
}

#[spacetimedb::reducer]
pub fn aoi_tick_reducer(ctx: &ReducerContext, _aoi_tick_timer: AoiTickTimer) -> Result<(), String> {
    // Only the server (module identity) may invoke the scheduled reducer.
    if ctx.sender != ctx.identity() {
        return Err("`tick` may not be invoked by clients.".into());
    }

    for actor in ctx.db.actor().is_player().filter(true) {
        // Guaranteed to be true, but we need to btree index a boolean to get performance improvements.
        // let ActorKind::Player(identity) = actor.kind else {
        //     continue;
        // };
        let Some(identity) = actor.identity else {
            log::error!("Actor {} is a player but has no identity", actor.id);
            continue;
        };

        ctx.db.actor_in_aoi().identity().delete(identity);
        let aoi_block: [u32; 9] = get_aoi_block(actor.cell_id);
        aoi_block
            .into_iter()
            .flat_map(|cell| ctx.db.actor().cell_id().filter(cell))
            .for_each(|a| {
                ctx.db.actor_in_aoi().insert(ActorInAoi {
                    id: 0,
                    identity,
                    transform_data_id: a.transform_data_id,
                    actor_id: a.id,
                });
            });
        log::info!("Computing AOI actors for player {}", actor.id);
    }

    Ok(())
}
