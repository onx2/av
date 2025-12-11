use bevy::{
    asset::embedded_asset,
    prelude::*,
    window::{
        CursorEntered, CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, WindowFocused,
        WindowResized,
    },
};

#[derive(Resource)]
struct CursorAssets {
    default: Handle<Image>,
    combat: Handle<Image>,
    ability: Handle<Image>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CursorKind {
    Default,
    Combat,
    Ability,
}

#[derive(Resource)]
struct CurrentCursor(CursorKind);

pub(super) fn plugin(app: &mut App) {
    embedded_asset!(app, "../assets/embedded/cursors/default.png");
    embedded_asset!(app, "../assets/embedded/cursors/ability.png");
    embedded_asset!(app, "../assets/embedded/cursors/combat.png");

    app.add_systems(Startup, load_cursor_assets);
    app.insert_resource(CurrentCursor(CursorKind::Default));

    // Apply when our desired cursor changes
    app.add_systems(
        Update,
        apply_cursor.run_if(resource_changed::<CurrentCursor>),
    );

    // Re-apply on focus/enter to avoid OS/browser resets
    app.add_systems(
        Update,
        (reapply_on_focus, reapply_on_enter, reapply_on_resize),
    );
}

fn load_cursor_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(CursorAssets {
        default: asset_server.load("embedded/cursors/default.png"),
        combat: asset_server.load("embedded/cursors/combat.png"),
        ability: asset_server.load("embedded/cursors/ability.png"),
    });
}

fn desired_handle(assets: &CursorAssets, kind: CursorKind) -> Handle<Image> {
    match kind {
        CursorKind::Default => assets.default.clone(),
        CursorKind::Combat => assets.combat.clone(),
        CursorKind::Ability => assets.ability.clone(),
    }
}

fn insert_cursor_icon(commands: &mut Commands, window: Entity, handle: Handle<Image>) {
    commands
        .entity(window)
        .insert(CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
            handle,
            texture_atlas: None,
            flip_x: false,
            flip_y: false,
            rect: None,
            hotspot: (0, 0),
        })));
}

fn apply_cursor(
    mut commands: Commands,
    window: Single<Entity, With<PrimaryWindow>>,
    assets: Res<CursorAssets>,
    current: Res<CurrentCursor>,
    q_icon: Query<Option<&CursorIcon>, With<PrimaryWindow>>,
) {
    // Only insert if missing or different
    let desired = CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
        handle: desired_handle(&assets, current.0),
        texture_atlas: None,
        flip_x: false,
        flip_y: false,
        rect: None,
        hotspot: (0, 0),
    }));

    let needs_update = match q_icon.get(*window) {
        Ok(Some(existing)) => existing != &desired,
        Ok(None) => true,
        Err(_) => true,
    };

    if needs_update {
        insert_cursor_icon(&mut commands, *window, desired_handle(&assets, current.0));
    }
}

fn reapply_on_focus(
    mut commands: Commands,
    mut messages: MessageReader<WindowFocused>,
    window: Single<Entity, With<PrimaryWindow>>,
    assets: Res<CursorAssets>,
    current: Res<CurrentCursor>,
) {
    for message in messages.read() {
        if message.focused {
            insert_cursor_icon(&mut commands, *window, desired_handle(&assets, current.0));
        }
    }
}

fn reapply_on_resize(
    mut commands: Commands,
    mut events: MessageReader<WindowResized>,
    window: Single<Entity, With<PrimaryWindow>>,
    assets: Res<CursorAssets>,
    current: Res<CurrentCursor>,
) {
    for _ in events.read() {
        insert_cursor_icon(&mut commands, *window, desired_handle(&assets, current.0));
    }
}

fn reapply_on_enter(
    mut commands: Commands,
    mut messages: MessageReader<CursorEntered>,
    window: Single<Entity, With<PrimaryWindow>>,
    assets: Res<CursorAssets>,
    current: Res<CurrentCursor>,
) {
    for _ in messages.read() {
        insert_cursor_icon(&mut commands, *window, desired_handle(&assets, current.0));
    }
}

// flip the desired cursor like this and the apply_cursor system will
// pick it up automatically due to resource_changed run condition.
// fn set_cursor_to_combat(mut cur: ResMut<CurrentCursor>) { cur.0 = CursorKind::Combat; }
