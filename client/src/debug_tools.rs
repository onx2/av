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
