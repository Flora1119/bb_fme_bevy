use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        ActivePlayWorld, CollectedStar, GameplayPhysicsPlugin, GridIndex, MapCamera,
        MapPresentationPlugin, MapSpawnPlugin, PHYSICS_HZ, PendingPlayInteractions, PlayHud,
        PlayHudPlugin, PlayInteraction, PlayRestartPlugin, PlaySession, PlaySessionPlugin,
        PlaySessionState, PlayWorld, PlayerBall, PlayerCameraPlugin, RestartPlayWorld,
        RuntimeBlock, SpawnValidatedMap, SpikeDeathPlugin, SpikeSensorCollider,
        StarCollectionPlugin,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const RESTART_MAP: &str = include_str!("../assets/maps/phase5_camera_follow_sandbox.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(RESTART_MAP).expect("restart map fixture must deserialize");

    ValidatedMap::from_document(&document, &config).expect("restart fixture must validate")
}

fn app_with_restartable_map() -> (App, ValidatedMap) {
    let map = load_validated_map();

    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        PlayHudPlugin,
        StarCollectionPlugin,
        SpikeDeathPlugin,
        MapPresentationPlugin,
        PlayerCameraPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(map.clone()));

    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    (app, map)
}

fn hud_text(app: &mut App) -> String {
    let world = app.world_mut();

    let mut huds = world.query_filtered::<&Text, With<PlayHud>>();

    let texts: Vec<String> = huds.iter(world).map(|text| text.0.clone()).collect();

    assert_eq!(texts.len(), 1);

    texts[0].clone()
}

fn active_root(app: &App) -> Entity {
    app.world()
        .resource::<ActivePlayWorld>()
        .root()
        .expect("active PlayWorld root must exist")
}

fn entity_count(app: &mut App) -> usize {
    let world = app.world_mut();

    let mut entities = world.query::<Entity>();

    entities.iter(world).count()
}

fn runtime_block_count(app: &mut App) -> usize {
    let world = app.world_mut();

    let mut blocks = world.query_filtered::<Entity, With<RuntimeBlock>>();

    blocks.iter(world).count()
}

fn play_world_count(app: &mut App) -> usize {
    let world = app.world_mut();

    let mut play_worlds = world.query_filtered::<Entity, With<PlayWorld>>();

    play_worlds.iter(world).count()
}

fn spike_sensor_count(app: &mut App) -> usize {
    let world = app.world_mut();

    let mut sensors = world.query_filtered::<Entity, With<SpikeSensorCollider>>();

    sensors.iter(world).count()
}

fn camera_position(app: &mut App) -> Vec2 {
    let world = app.world_mut();

    let mut cameras = world.query_filtered::<&Transform, With<MapCamera>>();

    let positions: Vec<Vec2> = cameras
        .iter(world)
        .map(|transform| transform.translation.truncate())
        .collect();

    assert_eq!(positions.len(), 1);

    positions[0]
}

#[test]
fn restart_restores_world_session_star_player_and_camera() {
    let (mut app, map) = app_with_restartable_map();

    let previous_root = active_root(&app);

    let previous_player = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(12, 2))
        .expect("player must be indexed");

    let previous_star = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(16, 2))
        .expect("star must be indexed");

    // 마지막 별을 먹어 클리어 상태를 만듭니다.
    app.world_mut()
        .resource_mut::<PendingPlayInteractions>()
        .push(PlayInteraction::collection(previous_star));

    app.update();

    let session = app.world().resource::<PlaySession>();

    assert_eq!(session.state(), PlaySessionState::Cleared);
    assert_eq!(session.collected_stars(), 1);

    assert!(app.world().get::<CollectedStar>(previous_star).is_some());

    let clear_time = session.elapsed_seconds();

    // Cleared 이후 시간이 정지하는지 확인합니다.
    for _ in 0..5 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<PlaySession>().elapsed_seconds(),
        clear_time
    );

    // 공의 위치와 속도를 일부러 크게 망가뜨립니다.
    app.world_mut()
        .get_mut::<Transform>(previous_player)
        .expect("player must have Transform")
        .translation = Vec3::new(30.0, 20.0, 0.0);

    app.world_mut()
        .get_mut::<LinearVelocity>(previous_player)
        .expect("player must have LinearVelocity")
        .0 = Vec2::new(12.0, 8.0);

    // 카메라도 일부러 다른 위치로 옮깁니다.
    let camera = {
        let world = app.world_mut();

        let mut cameras = world.query_filtered::<Entity, With<MapCamera>>();

        let entities: Vec<Entity> = cameras.iter(world).collect();

        assert_eq!(entities.len(), 1);

        entities[0]
    };

    app.world_mut()
        .get_mut::<Transform>(camera)
        .expect("camera must have Transform")
        .translation = Vec3::new(32.0, 22.0, 0.0);

    // restart command
    app.world_mut().write_message(RestartPlayWorld);

    app.update();

    let next_root = active_root(&app);

    assert_ne!(next_root, previous_root);

    assert!(
        !app.world().entities().contains(previous_root),
        "previous PlayWorld root must be despawned"
    );

    assert!(
        !app.world().entities().contains(previous_player),
        "previous player must be despawned"
    );

    assert!(
        !app.world().entities().contains(previous_star),
        "previous star must be despawned"
    );

    let play_world = app
        .world()
        .get::<PlayWorld>(next_root)
        .expect("new root must contain PlayWorld");

    assert_eq!(play_world.definition(), &map);

    let session = app.world().resource::<PlaySession>();

    assert_eq!(session.state(), PlaySessionState::Playing);
    assert_eq!(session.collected_stars(), 0);
    assert_eq!(session.elapsed_seconds(), 0.0);

    let new_player = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(12, 2))
        .expect("new player must be indexed");

    let new_star = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(16, 2))
        .expect("new star must be indexed");

    assert_ne!(new_player, previous_player);
    assert_ne!(new_star, previous_star);

    let player_transform = app
        .world()
        .get::<Transform>(new_player)
        .expect("new player must have Transform");

    assert_eq!(player_transform.translation, Vec3::new(12.0, 2.0, 0.0));

    let velocity = app
        .world()
        .get::<LinearVelocity>(new_player)
        .expect("new player must have LinearVelocity");

    assert_eq!(velocity.0, Vec2::ZERO);

    assert!(app.world().get::<CollectedStar>(new_star).is_none());
    assert!(app.world().get::<ColliderDisabled>(new_star).is_none());

    assert_eq!(camera_position(&mut app), Vec2::new(12.0, 7.0));
}

#[test]
fn restarting_same_map_100_times_does_not_leak_entities_or_state() {
    let (mut app, map) = app_with_restartable_map();

    let expected_entity_count = entity_count(&mut app);
    let expected_runtime_blocks = runtime_block_count(&mut app);
    let expected_spike_sensors = spike_sensor_count(&mut app);

    assert_eq!(expected_runtime_blocks, map.blocks.len());
    assert_eq!(play_world_count(&mut app), 1);

    let mut previous_root = active_root(&app);

    for restart_index in 0..100 {
        app.world_mut().write_message(RestartPlayWorld);

        app.update();

        let next_root = active_root(&app);

        assert_ne!(
            next_root, previous_root,
            "restart {restart_index} must create a new root"
        );

        assert!(
            !app.world().entities().contains(previous_root),
            "restart {restart_index} leaked the previous root"
        );

        assert_eq!(
            play_world_count(&mut app),
            1,
            "restart {restart_index} must leave exactly one PlayWorld"
        );

        assert_eq!(
            runtime_block_count(&mut app),
            expected_runtime_blocks,
            "restart {restart_index} leaked or lost RuntimeBlock entities"
        );

        assert_eq!(
            spike_sensor_count(&mut app),
            expected_spike_sensors,
            "restart {restart_index} leaked or lost spike sensors"
        );

        assert_eq!(
            entity_count(&mut app),
            expected_entity_count,
            "restart {restart_index} changed total entity count"
        );

        assert_eq!(
            app.world().resource::<GridIndex>().len(),
            map.blocks.len(),
            "restart {restart_index} corrupted GridIndex"
        );

        let session = app.world().resource::<PlaySession>();

        assert_eq!(session.state(), PlaySessionState::Playing);

        assert_eq!(session.collected_stars(), 0);
        assert_eq!(session.elapsed_seconds(), 0.0);

        previous_root = next_root;

        assert_eq!(
            hud_text(&mut app),
            "Stars: 0 / 1\nTime: 0.00\nState: Playing\nR: Restart"
        );
    }
}
