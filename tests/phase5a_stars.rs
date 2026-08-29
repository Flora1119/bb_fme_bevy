mod common;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        CollectedStar, CollectibleStar, GameplayPhysicsPlugin, GridIndex, MapSpawnPlugin,
        PHYSICS_HZ, PlayRestartPlugin, PlaySession, PlaySessionPlugin, RestartPlayWorld,
        STAR_SENSOR_RADIUS, SpawnValidatedMap, StarCollectionPlugin, StarSensorCollider,
        TransparentStar,
    },
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use common::load_validated_map;
use std::time::Duration;

const STAR_MAP: &str = include_str!("../assets/maps/phase5a_stars.json");

fn app_with_star_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        StarCollectionPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map(STAR_MAP)));

    app.update();
    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

fn entity_at(app: &App, x: i32, y: i32) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(x, y))
        .expect("expected indexed entity")
}

fn player(app: &App) -> Entity {
    entity_at(app, 2, 3)
}

fn set_player_state(app: &mut App, player: Entity, position: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity::ZERO,
    ));
}

fn run_ticks(app: &mut App, count: usize) {
    for _ in 0..count {
        app.update();
    }
}

fn raw_sensor_radius(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<Collider>(entity)
        .expect("star must have collider")
        .shape()
        .as_ball()
        .expect("star collider must be circular")
        .radius
}

fn assert_scale(app: &App, entity: Entity, expected: f32) {
    let transform = app
        .world()
        .get::<Transform>(entity)
        .expect("star must have Transform");

    assert_eq!(transform.scale, Vec3::new(expected, expected, 1.0,));
}

fn collected_count_at(position: Vec2) -> u32 {
    let mut app = app_with_star_map();

    app.world_mut().resource_mut::<Gravity>().0 = Vec2::ZERO;

    let player = player(&app);

    set_player_state(&mut app, player, position);

    run_ticks(&mut app, 4);

    app.world().resource::<PlaySession>().collected_stars()
}

#[test]
fn star_map_projects_default_and_explicit_scale_options() {
    let map = load_validated_map(STAR_MAP);

    assert_eq!(map.blocks.len(), 11);

    assert_eq!(map.settings.required_stars, 3);

    let default_star = map
        .block_at(GridPosition::new(3, 2))
        .expect("default star must exist");

    assert_eq!(default_star.options.len(), 1);

    assert_eq!(default_star.options[0].name, "Scale");

    assert_eq!(default_star.options[0].value, 1.0);

    let small_star = map
        .block_at(GridPosition::new(4, 2))
        .expect("small star must exist");

    assert_eq!(small_star.options.len(), 1);

    assert_eq!(small_star.options[0].name, "Scale");

    assert_eq!(small_star.options[0].value, 0.5);

    let transparent_star = map
        .block_at(GridPosition::new(5, 2))
        .expect("transparent star must exist");

    // 현재 block_assets_config에는
    // star_empty의 Scale 옵션 정의가 없습니다.
    assert!(transparent_star.options.is_empty());
}

#[test]
fn stars_use_the_unity_circle_and_scale_runtime_geometry() {
    let app = app_with_star_map();

    let normal = entity_at(&app, 3, 2);

    let small = entity_at(&app, 4, 2);

    let transparent = entity_at(&app, 5, 2);

    for star in [normal, small, transparent] {
        assert!(app.world().get::<CollectibleStar>(star,).is_some());

        assert!(app.world().get::<StarSensorCollider>(star,).is_some());

        assert!(app.world().get::<Sensor>(star).is_some());

        assert_eq!(raw_sensor_radius(&app, star,), STAR_SENSOR_RADIUS);
    }

    assert_scale(&app, normal, 1.0);

    assert_scale(&app, small, 0.5);

    assert_scale(&app, transparent, 1.0);

    assert!(app.world().get::<TransparentStar>(normal,).is_none());

    assert!(app.world().get::<TransparentStar>(small,).is_none());

    assert!(app.world().get::<TransparentStar>(transparent,).is_some());

    assert!(app.world().get::<ColliderDisabled>(normal,).is_none());

    assert!(app.world().get::<ColliderDisabled>(small,).is_none());

    assert!(app.world().get::<ColliderDisabled>(transparent,).is_some());
}

#[test]
fn scale_option_changes_the_actual_collection_area() {
    // 일반 별:
    //
    // star radius  = 0.4
    // player radius = 0.2
    //
    // 중심 거리 0.5이면 겹칩니다.
    assert_eq!(collected_count_at(Vec2::new(3.5, 2.0),), 1);

    // Scale 0.5 별:
    //
    // 실제 star radius = 0.2
    // player radius     = 0.2
    //
    // 중심 거리 0.5이면 닿지 않습니다.
    assert_eq!(collected_count_at(Vec2::new(4.5, 2.0),), 0);

    // 하지만 중심에 직접 들어가면
    // 작은 별도 정상적으로 획득됩니다.
    assert_eq!(collected_count_at(Vec2::new(4.0, 2.0),), 1);
}

#[test]
fn transparent_star_starts_dormant_and_cannot_be_collected() {
    let mut app = app_with_star_map();

    app.world_mut().resource_mut::<Gravity>().0 = Vec2::ZERO;

    let player = player(&app);

    let transparent = entity_at(&app, 5, 2);

    set_player_state(&mut app, player, Vec2::new(5.0, 2.0));

    run_ticks(&mut app, 4);

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 0);

    assert!(app.world().get::<CollectedStar>(transparent,).is_none());

    assert!(app.world().get::<TransparentStar>(transparent,).is_some());

    assert!(app.world().get::<ColliderDisabled>(transparent,).is_some());
}

#[test]
fn restart_restores_star_scale_collection_and_dormant_state() {
    let mut app = app_with_star_map();

    app.world_mut().resource_mut::<Gravity>().0 = Vec2::ZERO;

    let player = player(&app);

    let old_small = entity_at(&app, 4, 2);

    let old_transparent = entity_at(&app, 5, 2);

    set_player_state(&mut app, player, Vec2::new(4.0, 2.0));

    run_ticks(&mut app, 4);

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 1);

    assert!(app.world().get::<CollectedStar>(old_small,).is_some());

    assert!(app.world().get::<ColliderDisabled>(old_small,).is_some());

    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    assert!(!app.world().entities().contains(old_small));

    assert!(!app.world().entities().contains(old_transparent,));

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 0);

    let new_small = entity_at(&app, 4, 2);

    let new_transparent = entity_at(&app, 5, 2);

    assert_ne!(new_small, old_small);

    assert_ne!(new_transparent, old_transparent);

    assert!(app.world().get::<CollectedStar>(new_small,).is_none());

    assert!(app.world().get::<ColliderDisabled>(new_small,).is_none());

    assert_scale(&app, new_small, 0.5);

    assert_eq!(raw_sensor_radius(&app, new_small,), STAR_SENSOR_RADIUS);

    assert!(
        app.world()
            .get::<TransparentStar>(new_transparent,)
            .is_some()
    );

    assert!(
        app.world()
            .get::<ColliderDisabled>(new_transparent,)
            .is_some()
    );

    assert_scale(&app, new_transparent, 1.0);
}
