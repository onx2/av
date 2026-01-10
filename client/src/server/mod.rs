pub mod reducers;
pub mod types;

use crate::module_bindings::{
    AoiActorTableAccess, AoiTransformDataTableAccess, DbConnection, KccSettingsTableAccess,
    PlayerTableAccess, RemoteTables, SecondaryStatsTableAccess, WorldStaticTableAccess,
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
            .add_reducer::<EnterWorld>()
            .add_reducer::<LeaveWorld>()
            // --------------------------------
            // Register all tables
            // --------------------------------
            .add_table(RemoteTables::player)
            .add_view_with_pk(RemoteTables::aoi_actor, |a| a.id)
            .add_view_with_pk(RemoteTables::aoi_transform_data, |t| t.id)
            .add_table(RemoteTables::world_static)
            .add_table(RemoteTables::secondary_stats) /* TODO: Area of interest */
            .add_table(RemoteTables::kcc_settings)
            .with_run_fn(DbConnection::run_threaded),
    );
    app.add_systems(Update, on_connect);
}

fn on_connect(mut messages: ReadStdbConnectedMessage, stdb: SpacetimeDB) {
    for message in messages.read() {
        println!("SpacetimeDB module connected: {:?}", message.identity);

        stdb.subscription_builder().subscribe(vec![
            "SELECT * FROM player",
            "SELECT * FROM world_static",
            "SELECT * FROM kcc_settings",
            "SELECT * FROM secondary_stats", /* TODO: Area of interest */
            "SELECT * FROM aoi_actor",
            "SELECT * FROM aoi_transform_data",
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
            } else if key == "token-file" {
                return std::fs::read_to_string(arg)
                    .ok()
                    .map(|s| s.trim().to_string());
            }
        } else if arg == "--token" || arg == "-t" {
            pending_key = Some("token");
            continue;
        } else if let Some(val) = arg.strip_prefix("--token=") {
            return Some(val.to_string());
        } else if arg == "--token-file" {
            pending_key = Some("token-file");
            continue;
        } else if let Some(path) = arg.strip_prefix("--token-file=") {
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
