use avian2d::prelude::PhysicsDebugPlugin;
use bb_fme_bevy::gameplay::{
    BlockVisualPlugin, DevelopmentMapPlugin, GameplayPhysicsPlugin, MapPresentationPlugin,
    MapSpawnPlugin, PlaySessionPlugin, PlayerCameraPlugin, PlayerControlPlugin,
    StarCollectionPlugin,
};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            MapSpawnPlugin,
            GameplayPhysicsPlugin,
            PlaySessionPlugin,
            StarCollectionPlugin,
            PlayerControlPlugin,
            PhysicsDebugPlugin,
            BlockVisualPlugin,
            MapPresentationPlugin,
            PlayerCameraPlugin,
            DevelopmentMapPlugin,
        ))
        .run();
}
