mod common;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        GridIndex, MIN_VIEW_HEIGHT, MIN_VIEW_WIDTH, MapCamera, MapPresentationPlugin,
        MapSpawnPlugin, PLAYER_COLOR, PLAYER_VISUAL_SIZE, PLAYER_VISUAL_Z, PlaceholderVisual,
        SOLID_COLOR, SOLID_VISUAL_SIZE, SPIKE_COLOR, SPIKE_VISUAL_SIZE, STAR_COLOR,
        STAR_VISUAL_SIZE, SpawnValidatedMap,
    },
};
use bevy::{camera::ScalingMode, prelude::*};
use common::load_validated_map;

const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

fn app_with_presented_map() -> App {
    let mut app = App::new();

    app.add_plugins((MapSpawnPlugin, MapPresentationPlugin));
    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map(MINIMAL_MAP)));
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
        (GridPosition::new(2, 0), SOLID_COLOR, SOLID_VISUAL_SIZE),
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

#[test]
fn player_visual_is_rendered_in_front_of_map_blocks() {
    let app = app_with_presented_map();

    let index = app.world().resource::<GridIndex>();

    let player = index
        .entity_at(GridPosition::new(2, 2))
        .expect("player must exist");

    let block = index
        .entity_at(GridPosition::new(2, 0))
        .expect("block must exist");

    let player_z = app.world().get::<Transform>(player).unwrap().translation.z;

    let block_z = app.world().get::<Transform>(block).unwrap().translation.z;

    assert_eq!(player_z, PLAYER_VISUAL_Z);

    assert!(
        player_z > block_z,
        "player must render in front of map blocks"
    );
}
