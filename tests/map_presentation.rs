use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        GridIndex, MIN_VIEW_HEIGHT, MIN_VIEW_WIDTH, MapCamera, MapPresentationPlugin,
        MapSpawnPlugin, PLAYER_COLOR, PLAYER_VISUAL_SIZE, PlaceholderVisual, SOLID_COLOR,
        SOLID_VISUAL_SIZE, SPIKE_COLOR, SPIKE_VISUAL_SIZE, STAR_COLOR, STAR_VISUAL_SIZE,
        SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::{camera::ScalingMode, prelude::*};

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");
const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(MINIMAL_MAP).expect("map fixture must deserialize");

    ValidatedMap::from_document(&document, &config).expect("fixture must validate")
}

fn app_with_presented_map() -> App {
    let mut app = App::new();

    app.add_plugins((MapSpawnPlugin, MapPresentationPlugin));
    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));
    app.update();

    app
}

#[test]
fn camera_frames_the_twenty_five_by_fifteen_map() {
    let mut app = app_with_presented_map();
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<(&Transform, &Projection), With<MapCamera>>();
    let collected: Vec<_> = cameras.iter(world).collect();

    assert_eq!(collected.len(), 1);

    let (transform, projection) = collected[0];
    assert_eq!(transform.translation.truncate(), Vec2::new(12.0, 7.0));

    let Projection::Orthographic(orthographic) = projection else {
        panic!("map camera must use an orthographic projection");
    };

    assert!(matches!(
        orthographic.scaling_mode,
        ScalingMode::AutoMin {
            min_width,
            min_height,
        } if min_width == MIN_VIEW_WIDTH && min_height == MIN_VIEW_HEIGHT
    ));
}

#[test]
fn gameplay_roles_receive_distinct_placeholder_sprites() {
    let app = app_with_presented_map();
    let index = app.world().resource::<GridIndex>();

    let cases = [
        (GridPosition::new(2, 2), PLAYER_COLOR, PLAYER_VISUAL_SIZE),
        (GridPosition::new(8, 2), STAR_COLOR, STAR_VISUAL_SIZE),
        (GridPosition::new(5, 0), SOLID_COLOR, SOLID_VISUAL_SIZE),
        (GridPosition::new(12, 0), SPIKE_COLOR, SPIKE_VISUAL_SIZE),
    ];

    for (position, expected_color, expected_size) in cases {
        let entity = index
            .entity_at(position)
            .expect("fixture position must be indexed");
        let sprite = app
            .world()
            .get::<Sprite>(entity)
            .expect("gameplay role must receive a sprite");

        assert_eq!(sprite.color, expected_color);
        assert_eq!(sprite.custom_size, Some(expected_size));
        assert!(app.world().get::<PlaceholderVisual>(entity).is_some());
    }
}
