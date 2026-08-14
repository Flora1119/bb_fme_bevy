use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        BLOCK_WORLD_SIZE, BlockPhysicsBody, GameplayPhysicsPlugin, GridIndex, MapSpawnPlugin,
        PHYSICS_HZ, PLAYER_COLLIDER_RADIUS, PLAYER_GRAVITY_SCALE, PLAYER_MASS, PlayerPhysicsBody,
        SOLID_COLLIDER_SIZE, SPIKE_SENSOR_OFFSET, SPIKE_SENSOR_SIZE, SpawnValidatedMap,
        SpikeSensorCollider,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");
const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(MINIMAL_MAP).expect("map fixture must deserialize");

    ValidatedMap::from_document(&document, &config).expect("fixture must validate")
}

fn app_with_physics_bodies() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
    ));
    app.init_resource::<Assets<GizmoAsset>>();
    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    app.finish();
    app.cleanup();

    app.update();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

#[test]
fn normal_block_receives_a_static_full_tile_collider() {
    let mut app = app_with_physics_bodies();
    let block = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 0))
        .expect("normal block must be indexed");

    assert_eq!(
        app.world().get::<RigidBody>(block),
        Some(&RigidBody::Static)
    );
    assert!(app.world().get::<BlockPhysicsBody>(block).is_some());

    let collider = app
        .world()
        .get::<Collider>(block)
        .expect("normal block must have a collider");
    let cuboid = collider
        .shape()
        .as_cuboid()
        .expect("normal block collider must be rectangular");

    assert_eq!(cuboid.half_extents.x, SOLID_COLLIDER_SIZE.x * 0.5);
    assert_eq!(cuboid.half_extents.y, SOLID_COLLIDER_SIZE.y * 0.5);
    assert!(app.world().get::<Sensor>(block).is_none());
}

#[test]
fn normal_spike_uses_an_offset_sensor_child() {
    let mut app = app_with_physics_bodies();
    let spike = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(12, 0))
        .expect("normal spike must be indexed");

    assert_eq!(
        app.world().get::<RigidBody>(spike),
        Some(&RigidBody::Static)
    );
    assert!(app.world().get::<BlockPhysicsBody>(spike).is_some());
    assert!(app.world().get::<Collider>(spike).is_none());

    let world = app.world_mut();
    let mut sensors = world.query_filtered::<
        (Entity, &Transform, &ChildOf, &Collider),
        (With<SpikeSensorCollider>, With<Sensor>),
    >();
    let results: Vec<_> = sensors.iter(world).collect();

    assert_eq!(results.len(), 1);

    let (sensor, transform, child_of, collider) = results[0];

    assert_eq!(child_of.0, spike);
    assert_eq!(
        world
            .get::<ColliderOf>(sensor)
            .expect("spike sensor must attach to its rigid body")
            .body,
        spike
    );
    assert_eq!(transform.translation.truncate(), SPIKE_SENSOR_OFFSET);

    let cuboid = collider
        .shape()
        .as_cuboid()
        .expect("spike sensor must be rectangular");

    assert_eq!(cuboid.half_extents.x, SPIKE_SENSOR_SIZE.x * 0.5);
    assert_eq!(cuboid.half_extents.y, SPIKE_SENSOR_SIZE.y * 0.5);
}

#[test]
fn player_ball_receives_the_unity_baseline_dynamic_body() {
    let app = app_with_physics_bodies();
    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    assert_eq!(
        app.world().get::<RigidBody>(ball),
        Some(&RigidBody::Dynamic)
    );
    assert!(app.world().get::<PlayerPhysicsBody>(ball).is_some());
    assert_eq!(app.world().get::<Mass>(ball), Some(&Mass(PLAYER_MASS)));
    assert_eq!(
        app.world().get::<GravityScale>(ball),
        Some(&GravityScale(PLAYER_GRAVITY_SCALE))
    );
    assert_eq!(app.world().get::<Friction>(ball), Some(&Friction::ZERO));
    assert_eq!(
        app.world().get::<Restitution>(ball),
        Some(&Restitution::ZERO)
    );

    let locked_axes = app
        .world()
        .get::<LockedAxes>(ball)
        .expect("player rotation must be locked");

    assert_eq!(locked_axes.to_bits(), LockedAxes::ROTATION_LOCKED.to_bits());
    assert!(app.world().get::<NoAutoMass>(ball).is_some());
    assert!(app.world().get::<SweptCcd>(ball).is_some());
    assert!(app.world().get::<TransformInterpolation>(ball).is_some());
    assert!(app.world().get::<CollisionEventsEnabled>(ball).is_some());

    let collider = app
        .world()
        .get::<Collider>(ball)
        .expect("player ball must have a collider");
    let circle = collider
        .shape()
        .as_ball()
        .expect("player collider must be circular");

    assert_eq!(circle.radius, PLAYER_COLLIDER_RADIUS);
}

#[test]
fn player_ball_falls_and_settles_on_the_normal_block() {
    let mut app = app_with_physics_bodies();
    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    for _ in 0..100 {
        app.update();
    }

    let position = app
        .world()
        .get::<Position>(ball)
        .expect("physics must update the player position");
    let velocity = app
        .world()
        .get::<LinearVelocity>(ball)
        .expect("dynamic body must have a velocity");
    let expected_resting_y = 0.5 * BLOCK_WORLD_SIZE + PLAYER_COLLIDER_RADIUS;

    assert!(
        (position.0.y - expected_resting_y).abs() < 0.02,
        "expected y near {expected_resting_y}, found {}",
        position.0.y
    );
    assert!(
        velocity.0.y.abs() < 0.05,
        "expected settled vertical velocity, found {}",
        velocity.0.y
    );
}
