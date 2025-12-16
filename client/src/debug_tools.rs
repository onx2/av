//! Debug/performance tooling for native dev builds.
//!
//! This plugin is compiled/used only when the caller gates it behind `dev_native`
//! (recommended: `#[cfg(feature = "dev_native")] mod debug_tools;` in `main.rs`).

use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
use iyes_perf_ui::prelude::*;

/// Add debug/perf tooling (intended for `dev_native` builds only).
pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        SystemInformationDiagnosticsPlugin::default(),
        RenderDiagnosticsPlugin,
        PerfUiPlugin,
    ));

    app.add_systems(Startup, spawn_perf_ui);
}

fn spawn_perf_ui(mut commands: Commands) {
    commands.spawn(PerfUiAllEntries::default());
}
