use bb_fme_bevy::gameplay::MapSpawnPlugin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MapSpawnPlugin))
        .run();
}
