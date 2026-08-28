use avian2d::prelude::PhysicsDebugPlugin;
use bb_fme_bevy::gameplay::{
    BlockVisualPlugin, ClockBlockPlugin, DevelopmentMapPlugin, GameplayPhysicsPlugin,
    MapBoundaryPlugin, MapPresentationPlugin, MapSpawnPlugin, PlayHudPlugin, PlayRestartPlugin,
    PlaySessionPlugin, PlayerCameraPlugin, PlayerControlPlugin, SpikeDeathPlugin,
    StarCollectionPlugin, TeleportBlockPlugin,
};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins((
            MapSpawnPlugin,
            GameplayPhysicsPlugin,
            PlaySessionPlugin,
            PlayRestartPlugin,
            PlayHudPlugin,
            MapBoundaryPlugin,
            StarCollectionPlugin,
            BlockVisualPlugin,
            MapPresentationPlugin,
            PlayerCameraPlugin,
            DevelopmentMapPlugin,
        ))
        .add_plugins((
            SpikeDeathPlugin,
            ClockBlockPlugin,
            TeleportBlockPlugin,
            PlayerControlPlugin,
            PhysicsDebugPlugin,
        ))
        .run();
}
