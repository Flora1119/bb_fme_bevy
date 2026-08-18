use avian2d::prelude::PhysicsDebugPlugin;
use bb_fme_bevy::gameplay::{
    BlockVisualPlugin, DevelopmentMapPlugin, GameplayPhysicsPlugin, MapPresentationPlugin,
    MapSpawnPlugin, PlayHudPlugin, PlayRestartPlugin, PlaySessionPlugin, PlayerCameraPlugin,
    PlayerControlPlugin, SpikeDeathPlugin, StarCollectionPlugin,
};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            MapSpawnPlugin,
            GameplayPhysicsPlugin,
            PlaySessionPlugin,
            PlayRestartPlugin,
            PlayHudPlugin,
            StarCollectionPlugin,
            SpikeDeathPlugin,
            PlayerControlPlugin,
            PhysicsDebugPlugin,
            BlockVisualPlugin,
            MapPresentationPlugin,
            PlayerCameraPlugin,
            DevelopmentMapPlugin,
        ))
        .run();
}
