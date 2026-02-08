// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

#[cfg(feature = "dev_native")]
mod debug_tools;

mod actor;
mod camera;
mod cursor;
mod experience;
mod extrapolate_move;
mod health;
mod input;
mod level;
mod mana;
mod module_bindings;
mod movement;
// mod movement_state;
mod player;
mod secondary_stats;
mod server;
mod transform;
mod world;

pub use actor::{ActorEntity, ActorEntityMapping, LocalActor, RemoteActor, ensure_actor_entity};

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
            // transform::plugin,
            world::plugin,
            player::plugin,
            // extrapolate_move::plugin,
            health::plugin,
            mana::plugin,
            level::plugin,
            movement::plugin,
            camera::plugin,
            input::plugin,
            experience::plugin,
            cursor::plugin,
            actor::plugin,
            // movement_state::plugin,
            secondary_stats::plugin,
        ));

        #[cfg(feature = "dev_native")]
        app.add_plugins(debug_tools::plugin);
    }
}
