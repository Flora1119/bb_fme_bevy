use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        GridIndex, MapCamera, MapPresentationPlugin, MapSpawnPlugin, PlayerCameraPlugin,
        SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::prelude::*;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const CAMERA_MAP: &str = include_str!("../assets/maps/phase4_camera_follow_sandbox.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(CAMERA_MAP).expect("camera map fixture must deserialize");

    ValidatedMap::from_document(&document, &config).expect("camera fixture must validate")
}

fn app_with_camera_follow_map() -> App {
    let mut app = App::new();

    app.add_plugins((MapSpawnPlugin, MapPresentationPlugin, PlayerCameraPlugin));

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    app.update();

    app
}

fn set_player_position(app: &mut App, position: Vec2) {
    let player = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(12, 2))
        .expect("player must be indexed");

    app.world_mut()
        .get_mut::<Transform>(player)
        .expect("player must have a transform")
        .translation = position.extend(0.0);
}

fn camera_position(app: &mut App) -> Vec2 {
    let world = app.world_mut();

    let mut cameras = world.query_filtered::<&Transform, With<MapCamera>>();

    let positions: Vec<_> = cameras
        .iter(world)
        .map(|transform| transform.translation.truncate())
        .collect();

    assert_eq!(positions.len(), 1);

    positions[0]
}

#[test]
fn camera_is_clamped_to_the_map_edges() {
    let mut app = app_with_camera_follow_map();

    // 왼쪽 아래로 맵 범위를 훨씬 넘겨 봅니다.
    set_player_position(&mut app, Vec2::new(-100.0, -100.0));
    app.update();

    assert_eq!(camera_position(&mut app), Vec2::new(12.0, 7.0));

    // 반대로 오른쪽 위를 훨씬 넘깁니다.
    set_player_position(&mut app, Vec2::new(100.0, 100.0));
    app.update();

    assert_eq!(camera_position(&mut app), Vec2::new(32.0, 22.0));
}

#[test]
fn camera_tracks_horizontal_motion_but_respects_vertical_dead_zone() {
    let mut app = app_with_camera_follow_map();

    set_player_position(&mut app, Vec2::new(-100.0, -100.0));
    app.update();

    assert_eq!(camera_position(&mut app), Vec2::new(12.0, 7.0));

    set_player_position(&mut app, Vec2::new(20.0, 9.5));
    app.update();

    assert_eq!(camera_position(&mut app), Vec2::new(20.0, 7.0));
}

#[test]
fn camera_moves_when_player_leaves_vertical_dead_zone() {
    let mut app = app_with_camera_follow_map();

    set_player_position(&mut app, Vec2::new(21.0, 10.5));
    app.update();

    assert_eq!(camera_position(&mut app), Vec2::new(21.0, 7.5));
}
