pub mod reducers;
pub mod types;

use crate::module_bindings::{
    CharacterInstanceViewTableAccess, DbConnection, ExperienceViewTableAccess,
    HealthViewTableAccess, LevelViewTableAccess, ManaViewTableAccess, MovementStateViewTableAccess,
    PrimaryStatsViewTableAccess, RemoteTables, SecondaryStatsViewTableAccess,
    TransformViewTableAccess, WorldStaticTblTableAccess,
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadStdbConnectedMessage, StdbConnection, StdbPlugin};
use reducers::*;

pub type SpacetimeDB<'a> = Res<'a, StdbConnection<DbConnection>>;

pub(super) fn plugin(app: &mut App) {
    let token = read_token_from_cli_env();

    let stdb_plugin = StdbPlugin::default()
        .with_uri("http://127.0.0.1:3000")
        .with_module_name("av");

    let stdb_plugin = if let Some(tok) = token {
        info!("Using JWT from CLI/ENV for SpacetimeDB connection.");
        stdb_plugin.with_token(tok)
    } else {
        warn!("No JWT provided via CLI/ENV; identity will be ephemeral for this run.");
        stdb_plugin
    };

    app.add_plugins(
        stdb_plugin
            // --------------------------------
            // Register all reducers
            // --------------------------------
            .add_reducer::<RequestMove>()
            .add_reducer::<EnterGame>()
            .add_reducer::<CreateCharacter>()
            .add_reducer::<CancelMove>()
            // --------------------------------
            // Register all tables
            // --------------------------------
            .add_table(RemoteTables::world_static_tbl)
            .add_table_without_pk(RemoteTables::primary_stats_view)
            .add_view_with_pk(RemoteTables::secondary_stats_view, |r| r.actor_id)
            .add_view_with_pk(RemoteTables::movement_state_view, |r| r.actor_id)
            .add_view_with_pk(RemoteTables::health_view, |r| r.actor_id)
            .add_view_with_pk(RemoteTables::mana_view, |r| r.actor_id)
            .add_view_with_pk(RemoteTables::character_instance_view, |r| r.actor_id)
            .add_view_with_pk(RemoteTables::transform_view, |r| r.actor_id)
            .add_view_with_pk(RemoteTables::experience_view, |r| r.actor_id)
            .add_view_with_pk(RemoteTables::level_view, |r| r.actor_id)
            .with_run_fn(DbConnection::run_threaded),
    );
    app.add_systems(Update, on_connect);
}

fn on_connect(mut messages: ReadStdbConnectedMessage, stdb: SpacetimeDB) {
    for message in messages.read() {
        println!("SpacetimeDB module connected: {:?}", message.identity);

        stdb.subscription_builder().subscribe(vec![
            "SELECT * FROM primary_stats_view",
            "SELECT * FROM secondary_stats_view",
            "SELECT * FROM health_view",
            "SELECT * FROM mana_view",
            "SELECT * FROM experience_view",
            "SELECT * FROM level_view",
            "SELECT * FROM world_static_tbl",
            "SELECT * FROM movement_state_view",
            "SELECT * FROM character_instance_view",
            "SELECT * FROM transform_view",
        ]);
    }
}

/// Returns a JWT token from CLI args or environment if present.
///
/// Supported:
///   --token <JWT>
///   --token=<JWT>
///   --token-file <path>
///   --token-file=<path>
///   STDB_TOKEN or STDB_JWT environment variables
fn read_token_from_cli_env() -> Option<String> {
    // CLI: --token / --token=<JWT>
    let mut args = std::env::args().skip(1);
    let mut pending_key: Option<&'static str> = None;

    while let Some(arg) = args.next() {
        if let Some(key) = pending_key.take() {
            if key == "token" {
                return Some(arg);
            }

            if key == "token-file" {
                return std::fs::read_to_string(arg)
                    .ok()
                    .map(|s| s.trim().to_string());
            }
        }

        if arg == "--token" || arg == "-t" {
            pending_key = Some("token");
            continue;
        }

        if let Some(val) = arg.strip_prefix("--token=") {
            return Some(val.to_string());
        }

        if arg == "--token-file" {
            pending_key = Some("token-file");
            continue;
        }

        if let Some(path) = arg.strip_prefix("--token-file=") {
            return std::fs::read_to_string(path)
                .ok()
                .map(|s| s.trim().to_string());
        }
    }

    // ENV fallback: STDB_TOKEN or STDB_JWT
    std::env::var("STDB_TOKEN")
        .or_else(|_| std::env::var("STDB_JWT"))
        .ok()
}
