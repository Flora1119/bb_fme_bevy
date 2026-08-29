use bb_fme_bevy::gameplay::{
    ActivePlayWorld, DevelopmentMapPlugin, GridIndex, MapSpawnPlugin, PlayWorld,
};
use bevy::prelude::*;

#[test]
fn development_plugin_spawns_the_embedded_map_on_startup() {
    let mut app = App::new();

    app.add_plugins((MapSpawnPlugin, DevelopmentMapPlugin));

    app.update();

    let root = app
        .world()
        .resource::<ActivePlayWorld>()
        .root()
        .expect("development map root must exist after the first update");

    let play_world = app
        .world()
        .get::<PlayWorld>(root)
        .expect("active root must contain PlayWorld");

    let definition = play_world.definition();

    assert!(
        !definition.map_name.is_empty(),
        "development map must have a name"
    );

    assert!(
        !definition.blocks.is_empty(),
        "development map must contain blocks"
    );

    assert_eq!(
        app.world().resource::<GridIndex>().len(),
        definition.blocks.len(),
    );
}
