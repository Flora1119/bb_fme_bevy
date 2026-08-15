use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        BlockPhysicsBody, GameplayPhysicsPlugin, GridIndex, MIN_BOUNCE_VELOCITY, MapSpawnPlugin,
        PHYSICS_HZ, PLAYER_COLLIDER_RADIUS, PLAYER_GRAVITY_SCALE, PLAYER_MASS, PlayerPhysicsBody,
        SOLID_COLLIDER_SIZE, SPIKE_SENSOR_OFFSET, SPIKE_SENSOR_SIZE, SolidBlock, SpawnValidatedMap,
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
fn player_ball_repeatedly_bounces_from_the_normal_block() {
    let mut app = app_with_physics_bodies();
    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    let mut previous_vertical_velocity = 0.0_f32;
    let mut greatest_upward_velocity = f32::NEG_INFINITY;
    let mut bounce_count = 0_u32;

    // 50Hz 기준 400틱은 약 8초입니다.
    // 최소한 여러 차례의 바운스가 발생하는지 확인합니다.
    for _ in 0..400 {
        app.update();

        let vertical_velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("dynamic body must have a velocity")
            .0
            .y;

        greatest_upward_velocity = greatest_upward_velocity.max(vertical_velocity);

        // 낙하 중이던 속도가 최소 바운스 속도로 전환됐다면
        // 한 번의 바운스로 계산합니다.
        if previous_vertical_velocity < -0.1 && vertical_velocity >= MIN_BOUNCE_VELOCITY - 0.05 {
            bounce_count += 1;
        }

        previous_vertical_velocity = vertical_velocity;
    }

    assert!(
        bounce_count >= 3,
        "expected repeated floor bounces, observed {bounce_count}"
    );

    assert!(
        greatest_upward_velocity >= MIN_BOUNCE_VELOCITY - 0.05,
        "expected upward velocity near at least {MIN_BOUNCE_VELOCITY}, \
         found {greatest_upward_velocity}"
    );
}

#[test]
fn player_ball_stops_rising_when_it_hits_a_ceiling() {
    const CEILING_CENTER_Y: f32 = 3.0;
    const LAUNCH_SPEED: f32 = 12.0;
    const VELOCITY_TOLERANCE: f32 = 0.05;
    const POSITION_TOLERANCE: f32 = 0.05;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    // 테스트 전용 천장입니다.
    //
    // 개발용 JSON 맵은 수정하지 않고, 이 테스트 World에만
    // 정적 SolidBlock을 하나 추가합니다.
    app.world_mut().spawn((
        Name::new("Test ceiling"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(2.0, CEILING_CENTER_Y, 0.0),
    ));

    // 공을 천장 방향으로 발사합니다.
    app.world_mut()
        .get_mut::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0 = Vec2::new(0.0, LAUNCH_SPEED);

    let expected_contact_y =
        CEILING_CENTER_Y - SOLID_COLLIDER_SIZE.y * 0.5 - PLAYER_COLLIDER_RADIUS;

    let mut previous_vertical_velocity = LAUNCH_SPEED;
    let mut ceiling_stop = None;

    // 천장은 공 바로 위에 있으므로 30틱이면 충분합니다.
    for _ in 0..30 {
        app.update();

        let position = app
            .world()
            .get::<Position>(ball)
            .expect("physics must update the player position")
            .0;

        let vertical_velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("player must have a linear velocity")
            .0
            .y;

        // 상승 중이던 공의 상승 속도가 제거된 순간을 찾습니다.
        if previous_vertical_velocity > VELOCITY_TOLERANCE
            && vertical_velocity <= VELOCITY_TOLERANCE
        {
            ceiling_stop = Some((position.y, vertical_velocity));

            break;
        }

        previous_vertical_velocity = vertical_velocity;
    }

    let (stop_y, stop_velocity) = ceiling_stop.expect("player did not stop at the ceiling");

    assert!(
        (stop_y - expected_contact_y).abs() <= POSITION_TOLERANCE,
        "expected ceiling contact y near \
         {expected_contact_y}, found {stop_y}"
    );

    assert!(
        stop_velocity <= VELOCITY_TOLERANCE,
        "expected no remaining upward velocity, \
         found {stop_velocity}"
    );

    assert!(
        stop_velocity >= -1.0,
        "expected the ceiling to stop the player \
         without a strong downward rebound, \
         found {stop_velocity}"
    );
}

#[test]
#[ignore = "long-running: simulates ten minutes at 50 Hz"]
fn player_ball_bounces_stably_for_ten_simulated_minutes() {
    const SIMULATED_SECONDS: usize = 10 * 60;
    const MIN_EXPECTED_BOUNCES: u32 = 800;
    const BOUNCE_SPEED_TOLERANCE: f32 = 0.05;
    const MAX_TICKS_SINCE_LAST_BOUNCE: usize = 100;
    const MAX_HORIZONTAL_DRIFT: f32 = 0.01;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    let tick_count = SIMULATED_SECONDS * PHYSICS_HZ as usize;

    let mut previous_vertical_velocity = 0.0_f32;
    let mut bounce_count = 0_u32;
    let mut last_bounce_tick = None;

    let mut weakest_bounce = f32::INFINITY;
    let mut strongest_bounce = f32::NEG_INFINITY;

    let mut lowest_y = f32::INFINITY;
    let mut highest_y = f32::NEG_INFINITY;
    let mut greatest_horizontal_drift = 0.0_f32;

    for tick in 0..tick_count {
        app.update();

        let position = app
            .world()
            .get::<Position>(ball)
            .expect("physics must update the player position")
            .0;

        let velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("dynamic body must have a velocity")
            .0;

        assert!(
            position.x.is_finite() && position.y.is_finite(),
            "player position became non-finite at tick {tick}: {position:?}"
        );

        assert!(
            velocity.x.is_finite() && velocity.y.is_finite(),
            "player velocity became non-finite at tick {tick}: {velocity:?}"
        );

        lowest_y = lowest_y.min(position.y);
        highest_y = highest_y.max(position.y);

        greatest_horizontal_drift = greatest_horizontal_drift.max((position.x - 2.0).abs());

        let vertical_velocity = velocity.y;

        if previous_vertical_velocity < -0.1
            && vertical_velocity >= MIN_BOUNCE_VELOCITY - BOUNCE_SPEED_TOLERANCE
        {
            bounce_count += 1;
            last_bounce_tick = Some(tick);

            weakest_bounce = weakest_bounce.min(vertical_velocity);

            strongest_bounce = strongest_bounce.max(vertical_velocity);
        }

        previous_vertical_velocity = vertical_velocity;
    }

    assert!(
        bounce_count >= MIN_EXPECTED_BOUNCES,
        "expected at least {MIN_EXPECTED_BOUNCES} bounces, \
         observed {bounce_count}"
    );

    let last_bounce_tick = last_bounce_tick.expect("the player never bounced");

    let ticks_since_last_bounce = tick_count - 1 - last_bounce_tick;

    assert!(
        ticks_since_last_bounce <= MAX_TICKS_SINCE_LAST_BOUNCE,
        "the player appears to have stopped bouncing: \
         last bounce was {ticks_since_last_bounce} ticks before the end"
    );

    assert!(
        weakest_bounce >= MIN_BOUNCE_VELOCITY - BOUNCE_SPEED_TOLERANCE,
        "bounce speed became too weak: {weakest_bounce}"
    );

    assert!(
        strongest_bounce <= MIN_BOUNCE_VELOCITY + BOUNCE_SPEED_TOLERANCE,
        "bounce speed gained unexpected energy: {strongest_bounce}"
    );

    assert!(
        greatest_horizontal_drift <= MAX_HORIZONTAL_DRIFT,
        "player drifted horizontally by \
         {greatest_horizontal_drift}"
    );

    println!(
        "ten-minute stability result: \
         {bounce_count} bounces, \
         bounce speed {weakest_bounce:.4}..{strongest_bounce:.4}, \
         y range {lowest_y:.4}..{highest_y:.4}, \
         horizontal drift {greatest_horizontal_drift:.6}"
    );
}
