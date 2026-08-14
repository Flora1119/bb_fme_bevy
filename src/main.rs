use avian2d::prelude::PhysicsDebugPlugin;
use bb_fme_bevy::gameplay::{
    BlockVisualPlugin, DevelopmentMapPlugin, GameplayPhysicsPlugin, MapPresentationPlugin,
    MapSpawnPlugin,
};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            MapSpawnPlugin,
            GameplayPhysicsPlugin,
            PhysicsDebugPlugin,
            BlockVisualPlugin,
            MapPresentationPlugin,
            DevelopmentMapPlugin,
        ))
        .run();
}
