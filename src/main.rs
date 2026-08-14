use bb_fme_bevy::gameplay::{DevelopmentMapPlugin, MapPresentationPlugin, MapSpawnPlugin};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MapSpawnPlugin,
            MapPresentationPlugin,
            DevelopmentMapPlugin,
        ))
        .run();
}
