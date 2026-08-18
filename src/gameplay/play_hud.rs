use super::{ActivePlayWorld, MapSpawnSet, PlaySession, PlaySessionState, PlayWorld};
use bevy::prelude::*;

pub struct PlayHudPlugin;

impl Plugin for PlayHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_play_hud)
            .add_systems(Update, update_play_hud.after(MapSpawnSet));
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayHud;

fn spawn_play_hud(mut commands: Commands) {
    commands.spawn((
        Name::new("Play HUD"),
        PlayHud,
        Text::new("Loading..."),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor::WHITE,
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn update_play_hud(
    session: Res<PlaySession>,
    active_play_world: Res<ActivePlayWorld>,
    play_worlds: Query<&PlayWorld>,
    mut huds: Query<&mut Text, With<PlayHud>>,
) {
    let Some(root) = active_play_world.root() else {
        return;
    };

    let Ok(play_world) = play_worlds.get(root) else {
        return;
    };

    let required_stars = play_world.definition().settings.required_stars;

    let state_label = match session.state() {
        PlaySessionState::Playing => "Playing",
        PlaySessionState::Dead => "Dead",
        PlaySessionState::Cleared => "Clear",
    };

    let hud_text = format!(
        "Stars: {} / {}\nTime: {:.2}\nState: {}\nR: Restart",
        session.collected_stars(),
        required_stars,
        session.elapsed_seconds(),
        state_label,
    );

    for mut text in &mut huds {
        text.0.clone_from(&hud_text);
    }
}
