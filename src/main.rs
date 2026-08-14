use bb_fme_bevy::gameplay::{
    BlockVisualPlugin, DevelopmentMapPlugin, MapPresentationPlugin, MapSpawnPlugin,
};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            MapSpawnPlugin,
            BlockVisualPlugin,
            MapPresentationPlugin,
            DevelopmentMapPlugin,
        ))
        .run();
}
