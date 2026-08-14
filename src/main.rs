use bb_fme_bevy::gameplay::{DevelopmentMapPlugin, MapSpawnPlugin};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MapSpawnPlugin, DevelopmentMapPlugin))
        .run();
}
