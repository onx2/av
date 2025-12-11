// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

mod camera;
mod cursor;
mod module_bindings;
mod player;
mod server;
mod world;

#[cfg(target_os = "macos")]
use bevy::window::CompositeAlphaMode;

use bevy::picking::prelude::*;
use bevy::prelude::*;

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;
impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Window {
                    title: "Aurora's Veil".to_string(),
                    fit_canvas_to_parent: true,
                    transparent: true,
                    // decorations: false,
                    // resizable: false,
                    #[cfg(target_os = "macos")]
                    composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                    #[cfg(target_os = "linux")]
                    composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
                    ..default()
                }
                .into(),
                ..default()
            }),
            MeshPickingPlugin,
        ));

        app.add_plugins((
            server::plugin,
            player::plugin,
            camera::plugin,
            cursor::plugin,
            world::plugin,
        ));
    }
}
