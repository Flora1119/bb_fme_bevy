use crate::common::load_validated_map;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        ActivePlayWorld, FallingRock, FallingRockPlugin, GameplayPhysicsPlugin, GridIndex,
        MapBoundaryPlugin, MapSpawnPlugin, PHYSICS_HZ, PlayRestartPlugin, PlaySession,
        PlaySessionPlugin, PlaySessionState, RestartPlayWorld, SpawnValidatedMap,
        StarCollectionPlugin, StraightTransportPlugin,
    },
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BASE_MAP: &str = include_str!("../../assets/maps/synthetic_minimal_map.json");

const SPAWNER_POSITION: GridPosition = GridPosition::new(10, 8);
const PLAYER_POSITION: GridPosition = GridPosition::new(2, 2);
const SURFACE_POSITION: GridPosition = GridPosition::new(10, 2);

fn test_app(delay: f32, interval: f32, surface: Option<&str>) -> App {
    let mut json: serde_json::Value = serde_json::from_str(BASE_MAP).unwrap();

    let mut blocks = vec![
        serde_json::json!({
            "x": 2, "y": 2,
            "block": {
                "type": "item",
                "name": "ball",
                "dir": 0
            }
        }),
        serde_json::json!({
            "x": 23, "y": 12,
            "block": {
                "type": "item",
                "name": "star",
                "dir": 0
            }
        }),
        serde_json::json!({
            "x": 10, "y": 8,
            "block": {
                "type": "funcblock",
                "name": "fb_fr",
                "dir": 0
            }
        }),
    ];

    if let Some(block_id) = surface {
        let category = if block_id == "star" { "item" } else { "block" };

        blocks.push(serde_json::json!({
            "x": 10, "y": 2,
            "block": {
                "type": category,
                "name": block_id,
                "dir": 0
            }
        }));
    }

    json["blocks"] = serde_json::json!(blocks);
    json["map_settings"]["star_count"] =
        serde_json::json!(if surface == Some("star") { 2 } else { 1 });

    json["block_options"] = serde_json::json!([
        {
            "x": 10,
            "y": 8,
            "name": "fb_fr",
            "options": [
                { "value_name": "Delay", "value": delay },
                { "value_name": "Interval", "value": interval }
            ]
        }
    ]);

    let map = load_validated_map(&serde_json::to_string(&json).unwrap());

    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        MapBoundaryPlugin,
        StarCollectionPlugin,
        StraightTransportPlugin,
        FallingRockPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut().write_message(SpawnValidatedMap(map));
    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    // 각 테스트에서 접촉 위치를 정할 때까지 안전한 위치에 정지시킵니다.
    set_player(&mut app, Vec2::new(2.0, 12.0));

    app
}

fn entity_at(app: &App, position: GridPosition) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(position)
        .unwrap()
}

fn set_player(app: &mut App, position: Vec2) {
    let player = entity_at(app, PLAYER_POSITION);

    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(Vec2::ZERO),
        GravityScale(0.0),
    ));
}

fn rock_entities(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<FallingRock>>();

    query.iter(world).collect()
}

fn run_frames(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn wait_until(app: &mut App, message: &str, mut condition: impl FnMut(&mut App) -> bool) {
    for _ in 0..200 {
        if condition(app) {
            return;
        }

        app.update();
    }

    assert!(condition(app), "{message}");
}

fn first_rock(app: &mut App) -> Entity {
    wait_until(app, "첫 낙석이 생성되지 않았습니다", |app| {
        !rock_entities(app).is_empty()
    });

    rock_entities(app)[0]
}

fn wait_until_falling(app: &mut App, rock: Entity) {
    wait_until(
        app,
        "낙석이 낙하 상태로 바뀌지 않았습니다",
        |app| {
            app.world()
                .get::<FallingRock>(rock)
                .is_some_and(|rock| rock.is_falling())
        },
    );
}

#[test]
fn falling_rock_real_physics_respects_delay_fade_wait_and_interval() {
    let mut app = test_app(0.5, 1.0, None);

    run_frames(&mut app, 20);
    assert!(rock_entities(&mut app).is_empty());

    let rock = first_rock(&mut app);

    let first_spawn_time = app.world().resource::<PlaySession>().elapsed_seconds();

    assert!(
        (0.48..=0.56).contains(&first_spawn_time),
        "첫 생성 시간: {first_spawn_time}",
    );

    let root = app.world().resource::<ActivePlayWorld>().root().unwrap();

    let spawner = entity_at(&app, SPAWNER_POSITION);

    assert_eq!(
        app.world().get::<Visibility>(spawner),
        Some(&Visibility::Hidden),
    );

    // 숨겨진 생성기가 아닌 맵 루트의 자식이어야 합니다.
    assert_eq!(app.world().get::<ChildOf>(rock).unwrap().parent(), root,);

    run_frames(&mut app, 15);

    let state = app.world().get::<FallingRock>(rock).unwrap();

    assert!(!state.is_falling());
    assert!(state.alpha() > 0.4 && state.alpha() < 0.9);

    assert!(
        app.world()
            .get::<Position>(rock)
            .unwrap()
            .0
            .distance(Vec2::new(10.0, 8.0))
            < 0.001,
    );

    assert_eq!(app.world().get::<GravityScale>(rock).unwrap().0, 0.0,);

    wait_until_falling(&mut app, rock);

    assert_eq!(app.world().get::<FallingRock>(rock).unwrap().alpha(), 1.0,);
    assert_eq!(app.world().get::<GravityScale>(rock).unwrap().0, 1.0,);

    run_frames(&mut app, 10);

    assert!(app.world().get::<Position>(rock).unwrap().y < 8.0);
    assert!(app.world().get::<LinearVelocity>(rock).unwrap().y < 0.0);

    wait_until(
        &mut app,
        "다음 낙석이 생성되지 않았습니다",
        |app| rock_entities(app).len() == 2,
    );

    let second_spawn_time = app.world().resource::<PlaySession>().elapsed_seconds();

    let interval = second_spawn_time - first_spawn_time;

    assert!(
        (0.98..=1.04).contains(&interval),
        "실제 생성 간격: {interval}",
    );

    // 동적으로 생성한 낙석은 맵의 점유 칸에 추가하지 않습니다.
    assert_eq!(app.world().resource::<GridIndex>().len(), 3);
}

#[test]
fn falling_rock_fade_contact_kills_player_without_removing_rock() {
    let mut app = test_app(0.0, 10.0, None);
    let rock = first_rock(&mut app);
    let position = app.world().get::<Position>(rock).unwrap().0;

    assert!(!app.world().get::<FallingRock>(rock).unwrap().is_falling());

    set_player(&mut app, position);

    wait_until(
        &mut app,
        "나타나는 중인 낙석이 공을 사망시키지 않았습니다",
        |app| app.world().resource::<PlaySession>().state() == PlaySessionState::Dead,
    );

    assert!(app.world().entities().contains(rock));
    assert!(!app.world().get::<FallingRock>(rock).unwrap().is_falling());
}

#[test]
fn falling_rock_falling_contact_kills_player_and_removes_rock() {
    let mut app = test_app(0.0, 10.0, None);
    let rock = first_rock(&mut app);

    wait_until_falling(&mut app, rock);

    let position = app.world().get::<Position>(rock).unwrap().0;
    set_player(&mut app, position - Vec2::new(0.0, 2.0));

    wait_until(
        &mut app,
        "떨어지는 낙석이 공을 사망시키지 않았습니다",
        |app| app.world().resource::<PlaySession>().state() == PlaySessionState::Dead,
    );

    assert!(!app.world().entities().contains(rock));
}

#[test]
fn falling_rock_is_removed_by_solid_child_collider_and_item_sensor() {
    for surface in ["b_normal", "b_o_half", "star"] {
        let mut app = test_app(0.0, 10.0, Some(surface));
        let surface_entity = entity_at(&app, SURFACE_POSITION);
        let rock = first_rock(&mut app);

        wait_until_falling(&mut app, rock);

        // 낙석이 표면 근처까지 실제로 내려오는지 먼저 확인합니다.
        wait_until(
            &mut app,
            "낙석이 표면까지 내려오지 않았습니다",
            |app| {
                app.world()
                    .get::<Position>(rock)
                    .is_some_and(|position| position.y < 4.5)
            },
        );

        wait_until(
            &mut app,
            "표면 접촉 후 낙석이 제거되지 않았습니다",
            |app| !app.world().entities().contains(rock),
        );

        assert!(app.world().entities().contains(surface_entity));
        assert_eq!(
            app.world()
                .resource::<GridIndex>()
                .entity_at(SURFACE_POSITION),
            Some(surface_entity),
        );
        assert_eq!(
            app.world().resource::<PlaySession>().state(),
            PlaySessionState::Playing,
        );
    }
}

#[test]
fn falling_rock_is_removed_by_map_boundary_without_a_floor() {
    let mut app = test_app(0.0, 10.0, None);
    let rock = first_rock(&mut app);

    wait_until_falling(&mut app, rock);

    wait_until(
        &mut app,
        "낙석이 맵 아래쪽으로 내려오지 않았습니다",
        |app| {
            app.world()
                .get::<Position>(rock)
                .is_some_and(|position| position.y < 0.0)
        },
    );

    wait_until(
        &mut app,
        "맵 경계에서 낙석이 제거되지 않았습니다",
        |app| !app.world().entities().contains(rock),
    );

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing,
    );
}

#[test]
fn falling_rock_restart_removes_old_rocks_and_restarts_delay() {
    let mut app = test_app(0.5, 1.0, None);

    let old_root = app.world().resource::<ActivePlayWorld>().root().unwrap();

    let old_spawner = entity_at(&app, SPAWNER_POSITION);
    let old_rock = first_rock(&mut app);

    wait_until_falling(&mut app, old_rock);

    app.world_mut().write_message(RestartPlayWorld);
    app.update();

    let new_root = app.world().resource::<ActivePlayWorld>().root().unwrap();

    assert_ne!(new_root, old_root);
    assert!(!app.world().entities().contains(old_root));
    assert!(!app.world().entities().contains(old_spawner));
    assert!(!app.world().entities().contains(old_rock));

    set_player(&mut app, Vec2::new(2.0, 12.0));
    run_frames(&mut app, 20);

    // 재시작 후에도 최초 대기 시간을 다시 기다려야 합니다.
    assert!(rock_entities(&mut app).is_empty());

    let new_rock = first_rock(&mut app);

    assert_ne!(new_rock, old_rock);
    assert!(
        !app.world()
            .get::<FallingRock>(new_rock)
            .unwrap()
            .is_falling()
    );

    assert!(
        app.world()
            .get::<Position>(new_rock)
            .unwrap()
            .0
            .distance(Vec2::new(10.0, 8.0))
            < 0.001,
    );
}

#[test]
fn falling_rock_sandbox_map_validates() {
    load_validated_map(include_str!(
        "../../assets/maps/phase5d_falling_rock_sandbox.json"
    ));
}
