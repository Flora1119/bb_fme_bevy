use avian2d::prelude::PhysicsDebugPlugin;
use bb_fme_bevy::gameplay::{
    BlockVisualPlugin, DevelopmentMapPlugin, GameplayPlugin, MapPresentationPlugin, PlayHudPlugin,
    PlayerCameraPlugin,
};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(GameplayPlugin)
        .add_plugins((
            PlayHudPlugin,
            BlockVisualPlugin,
            MapPresentationPlugin,
            PlayerCameraPlugin,
            DevelopmentMapPlugin,
            PhysicsDebugPlugin,
        ))
        .run();
}
