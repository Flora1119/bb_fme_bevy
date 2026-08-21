use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        CollectibleStar, DeadlySpike, GameplayPhysicsPlugin, GridIndex, JumpBlock, MapSpawnPlugin,
        PHYSICS_HZ, PendingPlayInteractions, PlayInteraction, PlayRestartPlugin, PlaySession,
        PlaySessionPlugin, PlaySessionState, PlayerBall, RestartPlayWorld, SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const UNITY_PLAYTHROUGH_MAP: &str = include_str!("../assets/maps/unity_phase4_playthrough.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument = serde_json::from_str(UNITY_PLAYTHROUGH_MAP)
        .expect("Unity playthrough fixture must deserialize");

    ValidatedMap::from_document(&document, &config)
        .expect("Unity playthrough fixture must validate")
}

fn app_with_vertical_slice() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

#[test]
fn unity_playthrough_fixture_supports_death_restart_and_clear_cycle() {
    let mut app = app_with_vertical_slice();

    let player = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(6, 2))
        .expect("player must exist");

    let spike = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(3, 1))
        .expect("spike must exist");

    let jump = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(9, 0))
        .expect("jump block must exist");

    let star = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(13, 2))
        .expect("star must exist");

    assert!(app.world().get::<PlayerBall>(player).is_some());

    assert!(app.world().get::<DeadlySpike>(spike).is_some());

    assert!(app.world().get::<JumpBlock>(jump).is_some());

    assert!(app.world().get::<CollectibleStar>(star).is_some());

    // Playing 상태에서는 시간이 증가합니다.
    app.update();

    assert!(app.world().resource::<PlaySession>().elapsed_seconds() > 0.0);

    // 첫 번째 런: 사망.
    app.world_mut()
        .resource_mut::<PendingPlayInteractions>()
        .push(PlayInteraction::death(spike));

    app.update();

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Dead
    );

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 0);

    let death_time = app.world().resource::<PlaySession>().elapsed_seconds();

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<PlaySession>().elapsed_seconds(),
        death_time
    );

    // 재시작.
    app.world_mut().write_message(RestartPlayWorld);

    app.update();

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing
    );

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 0);

    assert_eq!(app.world().resource::<PlaySession>().elapsed_seconds(), 0.0);

    // PlayWorld가 재생성됐으므로
    // 별 Entity도 새로 얻어야 합니다.
    let restarted_star = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(13, 2))
        .expect("star must exist after restart");

    assert_ne!(restarted_star, star);

    // 두 번째 런: 마지막 별 수집.
    app.world_mut()
        .resource_mut::<PendingPlayInteractions>()
        .push(PlayInteraction::collection(restarted_star));

    app.update();

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Cleared
    );

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 1);

    let clear_time = app.world().resource::<PlaySession>().elapsed_seconds();

    for _ in 0..5 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<PlaySession>().elapsed_seconds(),
        clear_time
    );
}
