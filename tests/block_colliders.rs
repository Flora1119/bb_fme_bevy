use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        BlockPhysicsBody, GameplayPhysicsPlugin, GridIndex, MIN_BOUNCE_VELOCITY,
        MIN_WALL_BOUNCE_SPEED, MapSpawnPlugin, PHYSICS_HZ, PLAYER_COLLIDER_RADIUS,
        PLAYER_GRAVITY_SCALE, PLAYER_MASS, PlayerPhysicsBody, SOLID_COLLIDER_SIZE,
        SPIKE_SENSOR_OFFSET, SPIKE_SENSOR_SIZE, SolidBlock, SpawnValidatedMap, SpikeSensorCollider,
        WALL_BOUNCE_DAMPING_RATIO,
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
fn player_ball_bounces_away_from_a_right_wall_with_minimum_speed() {
    const WALL_CENTER_X: f32 = 3.0;
    const LAUNCH_SPEED: f32 = 5.0;
    const VELOCITY_TOLERANCE: f32 = 0.05;
    const POSITION_TOLERANCE: f32 = 0.08;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    // 공 오른쪽에 테스트용 수직 벽을 만듭니다.
    app.world_mut().spawn((
        Name::new("Test right wall"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(WALL_CENTER_X, 2.0, 0.0),
    ));

    // 낮은 속도로 오른쪽 벽을 향해 발사합니다.
    app.world_mut()
        .get_mut::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0 = Vec2::new(LAUNCH_SPEED, 0.0);

    let expected_contact_x = WALL_CENTER_X - SOLID_COLLIDER_SIZE.x * 0.5 - PLAYER_COLLIDER_RADIUS;

    let mut wall_bounce = None;

    for _ in 0..20 {
        app.update();

        let position = app
            .world()
            .get::<Position>(ball)
            .expect("physics must update the player position")
            .0;

        let horizontal_velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("player must have a linear velocity")
            .0
            .x;

        if horizontal_velocity < -1.0 {
            wall_bounce = Some((position.x, horizontal_velocity));

            break;
        }
    }

    let (contact_x, rebound_velocity) =
        wall_bounce.expect("player did not bounce from the right wall");

    assert!(
        (contact_x - expected_contact_x).abs() <= POSITION_TOLERANCE,
        "expected right wall contact x near \
         {expected_contact_x}, found {contact_x}"
    );

    assert!(
        (rebound_velocity + MIN_WALL_BOUNCE_SPEED).abs() <= VELOCITY_TOLERANCE,
        "expected right wall rebound velocity near \
         -{MIN_WALL_BOUNCE_SPEED}, \
         found {rebound_velocity}"
    );
}

#[test]
fn player_ball_damps_a_high_speed_left_wall_impact() {
    const WALL_CENTER_X: f32 = 1.0;
    const LAUNCH_SPEED: f32 = 20.0;
    const VELOCITY_TOLERANCE: f32 = 0.1;
    const POSITION_TOLERANCE: f32 = 0.08;

    let expected_rebound_speed = LAUNCH_SPEED * WALL_BOUNCE_DAMPING_RATIO;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    // 공 왼쪽에 테스트용 수직 벽을 만듭니다.
    app.world_mut().spawn((
        Name::new("Test left wall"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(WALL_CENTER_X, 2.0, 0.0),
    ));

    // 고속으로 왼쪽 벽을 향해 발사합니다.
    app.world_mut()
        .get_mut::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0 = Vec2::new(-LAUNCH_SPEED, 0.0);

    let expected_contact_x = WALL_CENTER_X + SOLID_COLLIDER_SIZE.x * 0.5 + PLAYER_COLLIDER_RADIUS;

    let mut wall_bounce = None;

    for _ in 0..20 {
        app.update();

        let position = app
            .world()
            .get::<Position>(ball)
            .expect("physics must update the player position")
            .0;

        let horizontal_velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("player must have a linear velocity")
            .0
            .x;

        if horizontal_velocity > 1.0 {
            wall_bounce = Some((position.x, horizontal_velocity));

            break;
        }
    }

    let (contact_x, rebound_velocity) =
        wall_bounce.expect("player did not bounce from the left wall");

    assert!(
        (contact_x - expected_contact_x).abs() <= POSITION_TOLERANCE,
        "expected left wall contact x near \
         {expected_contact_x}, found {contact_x}"
    );

    assert!(
        (rebound_velocity - expected_rebound_speed).abs() <= VELOCITY_TOLERANCE,
        "expected damped rebound velocity near \
         {expected_rebound_speed}, \
         found {rebound_velocity}"
    );
}

#[test]
fn player_ball_does_not_wall_bounce_while_grazing() {
    // 공의 오른쪽 가장자리와 벽이 아주 조금 겹치게 하여
    // 확실하게 접촉이 생성되도록 합니다.
    const WALL_CENTER_X: f32 = 2.699;
    const MAX_HORIZONTAL_SPEED: f32 = 0.25;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    app.world_mut().spawn((
        Name::new("Test grazing wall"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(WALL_CENTER_X, 2.0, 0.0),
    ));

    // 벽 쪽 수평 속도 없이 벽을 따라 아래로 움직입니다.
    app.world_mut()
        .get_mut::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0 = Vec2::new(0.0, -2.0);

    for _ in 0..3 {
        app.update();
    }

    let velocity = app
        .world()
        .get::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0;

    assert!(
        velocity.x.abs() <= MAX_HORIZONTAL_SPEED,
        "grazing contact unexpectedly launched the player \
         horizontally: {}",
        velocity.x
    );
}

#[test]
fn player_ball_is_not_accelerated_while_moving_away_from_a_wall() {
    const WALL_CENTER_X: f32 = 2.699;
    const MOVING_AWAY_SPEED: f32 = -0.01;
    const MAX_ALLOWED_REBOUND_SPEED: f32 = 0.5;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    app.world_mut().spawn((
        Name::new("Test separating wall"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(WALL_CENTER_X, 2.0, 0.0),
    ));

    // 벽은 오른쪽에 있지만 공은 이미 왼쪽으로
    // 아주 천천히 빠져나가는 중입니다.
    app.world_mut()
        .get_mut::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0 = Vec2::new(MOVING_AWAY_SPEED, 0.0);

    app.update();

    let horizontal_velocity = app
        .world()
        .get::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0
        .x;

    assert!(
        horizontal_velocity.abs() <= MAX_ALLOWED_REBOUND_SPEED,
        "separating contact unexpectedly amplified \
         horizontal velocity to {horizontal_velocity}"
    );
}

#[test]
fn floor_contact_wins_over_wall_contact_at_a_corner() {
    const FLOOR_CENTER_Y: f32 = 1.0;
    const WALL_CENTER_X: f32 = 3.0;
    const APPROACH_SPEED: f32 = 5.0;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    // 중력에 의한 속도 변화 없이 정확한 대각선으로
    // 모서리에 진입시키기 위해 이 테스트에서만 중력을 끕니다.
    *app.world_mut()
        .get_mut::<GravityScale>(ball)
        .expect("player must have a gravity scale") = GravityScale(0.0);

    // 공 아래쪽의 테스트용 바닥입니다.
    app.world_mut().spawn((
        Name::new("Test corner floor"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(2.0, FLOOR_CENTER_Y, 0.0),
    ));

    // 바닥 오른쪽 끝과 맞닿는 테스트용 벽입니다.
    app.world_mut().spawn((
        Name::new("Test corner wall"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(WALL_CENTER_X, 2.0, 0.0),
    ));

    app.world_mut()
        .get_mut::<LinearVelocity>(ball)
        .expect("player must have a linear velocity")
        .0 = Vec2::new(APPROACH_SPEED, -APPROACH_SPEED);

    let mut corner_response = None;

    for _ in 0..10 {
        app.update();

        let velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("player must have a linear velocity")
            .0;

        if velocity.y >= MIN_BOUNCE_VELOCITY - 0.05 {
            corner_response = Some(velocity);
            break;
        }
    }

    let velocity = corner_response.expect("player did not receive a floor response at the corner");

    assert!(
        velocity.y >= MIN_BOUNCE_VELOCITY - 0.05,
        "expected floor bounce at the corner, \
         found vertical velocity {}",
        velocity.y
    );

    assert!(
        velocity.x > -MIN_WALL_BOUNCE_SPEED * 0.5,
        "wall response overrode floor priority \
         at the corner: horizontal velocity {}",
        velocity.x
    );
}

#[test]
fn swept_ccd_prevents_high_speed_wall_tunneling() {
    const START_X: f32 = 0.0;
    const START_Y: f32 = 2.0;

    const WALL_CENTER_X: f32 = 6.0;
    const WALL_THICKNESS: f32 = 0.05;
    const WALL_HEIGHT: f32 = 2.0;

    // 50Hz에서는 한 물리 틱에 4칸을 이동합니다.
    //
    // 시작 위치가 x=2이고 벽이 x=4이므로,
    // CCD가 없으면 공은 한 틱 만에 벽 너머 x=6까지 이동합니다.
    const LAUNCH_SPEED: f32 = 200.0;

    const POSITION_TOLERANCE: f32 = 0.1;
    const VELOCITY_TOLERANCE: f32 = 0.5;

    let expected_rebound_speed = LAUNCH_SPEED * WALL_BOUNCE_DAMPING_RATIO;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    assert!(
        app.world().get::<SweptCcd>(ball).is_some(),
        "player must have SweptCcd enabled"
    );

    // 이번 테스트에서는 Speculative Collision을 끕니다.
    //
    // 따라서 얇은 벽 관통을 막는 역할은
    // SweptCcd가 단독으로 담당해야 합니다.
    app.world_mut()
        .resource_mut::<NarrowPhaseConfig>()
        .default_speculative_margin = 0.0;

    // 공을 정확한 시작 위치에 놓고 중력을 끈 뒤,
    // 오른쪽으로 극고속 발사합니다.
    app.world_mut().entity_mut(ball).insert((
        Position(Vec2::new(START_X, START_Y)),
        Transform::from_xyz(START_X, START_Y, 0.0),
        LinearVelocity(Vec2::new(LAUNCH_SPEED, 0.0)),
        GravityScale(0.0),
    ));

    // 일반 블록보다 훨씬 얇은 테스트용 벽입니다.
    //
    // BlockPhysicsBody를 함께 넣어야
    // attach_block_colliders가 전체 타일 Collider를
    // 추가하지 않습니다.
    app.world_mut().spawn((
        Name::new("Test ultra-thin wall"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(WALL_THICKNESS, WALL_HEIGHT),
        Transform::from_xyz(WALL_CENTER_X, START_Y, 0.0),
    ));

    let expected_contact_x = WALL_CENTER_X - WALL_THICKNESS * 0.5 - PLAYER_COLLIDER_RADIUS;

    let wall_far_face_x = WALL_CENTER_X + WALL_THICKNESS * 0.5;

    // 첫 번째 고속 틱은 충돌 후보 AABB를 준비하는 틱입니다.
    //
    // 공은 x=0에서 x=4로 이동하며, 아직 x=6의 벽에는
    // 도달하지 않아야 합니다. 이 틱이 끝날 때 Avian이
    // 다음 이동 경로를 위한 속도 확장 AABB를 준비합니다.
    app.update();

    let primed_position = app
        .world()
        .get::<Position>(ball)
        .expect("physics must update the player position")
        .0;

    assert!(
        primed_position.x < expected_contact_x,
        "preparation tick reached the wall too early: \
     x={}, expected contact x={expected_contact_x}",
        primed_position.x
    );

    let mut furthest_x = primed_position.x;
    let mut wall_bounce = None;

    // 이제 다음 틱의 x=4 → x=8 이동 경로가
    // 얇은 벽 x=6을 지나므로 Swept CCD가 관통을 막아야 합니다.
    //
    // CCD가 충돌 시점으로 되돌린 뒤 실제 CollisionStart가
    // 다음 틱에 만들어질 수도 있으므로 여유 있게 실행합니다.
    for tick in 1..8 {
        app.update();

        let position = app
            .world()
            .get::<Position>(ball)
            .expect("physics must update the player position")
            .0;

        let velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("player must have a linear velocity")
            .0;

        furthest_x = furthest_x.max(position.x);

        // 공의 중심이 벽 반대편 면을 넘어갔다면
        // 얇은 벽을 관통한 것입니다.
        assert!(
            position.x < wall_far_face_x,
            "player tunneled through the thin wall \
             at tick {tick}: x={}, wall far face={wall_far_face_x}",
            position.x
        );

        if velocity.x < -1.0 {
            wall_bounce = Some((tick, position.x, velocity.x));

            break;
        }
    }

    let (bounce_tick, contact_x, rebound_velocity) =
        wall_bounce.expect("SweptCcd prevented no usable wall response");

    assert!(
        (contact_x - expected_contact_x).abs() <= POSITION_TOLERANCE,
        "expected high-speed wall contact x near \
         {expected_contact_x}, found {contact_x}"
    );

    assert!(
        (rebound_velocity + expected_rebound_speed).abs() <= VELOCITY_TOLERANCE,
        "expected high-speed rebound velocity near \
         -{expected_rebound_speed}, \
         found {rebound_velocity}"
    );

    println!(
        "swept CCD wall result: \
         bounce tick {bounce_tick}, \
         furthest x {furthest_x:.4}, \
         contact x {contact_x:.4}, \
         rebound velocity {rebound_velocity:.4}"
    );
}

#[test]
fn swept_ccd_prevents_high_speed_floor_tunneling() {
    const START_X: f32 = 6.0;
    const START_Y: f32 = 8.0;

    const FLOOR_CENTER_Y: f32 = 2.0;
    const FLOOR_THICKNESS: f32 = 0.05;
    const FLOOR_WIDTH: f32 = 2.0;

    // 50Hz에서 한 틱에 아래쪽으로 4칸 이동합니다.
    const FALL_SPEED: f32 = 200.0;

    const POSITION_TOLERANCE: f32 = 0.1;
    const VELOCITY_TOLERANCE: f32 = 0.1;

    let mut app = app_with_physics_bodies();

    let ball = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed");

    assert!(
        app.world().get::<SweptCcd>(ball).is_some(),
        "player must have SweptCcd enabled"
    );

    // 벽 테스트와 마찬가지로 예측 접촉을 끄고
    // SweptCcd만으로 관통을 방지하는지 확인합니다.
    app.world_mut()
        .resource_mut::<NarrowPhaseConfig>()
        .default_speculative_margin = 0.0;

    // 기존 맵의 일반 바닥과 겹치지 않도록
    // 공을 x=6 위치로 옮깁니다.
    app.world_mut().entity_mut(ball).insert((
        Position(Vec2::new(START_X, START_Y)),
        Transform::from_xyz(START_X, START_Y, 0.0),
        LinearVelocity(Vec2::new(0.0, -FALL_SPEED)),
        GravityScale(0.0),
    ));

    // 높이가 0.05칸뿐인 매우 얇은 바닥입니다.
    app.world_mut().spawn((
        Name::new("Test ultra-thin floor"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(FLOOR_WIDTH, FLOOR_THICKNESS),
        Transform::from_xyz(START_X, FLOOR_CENTER_Y, 0.0),
    ));

    let expected_contact_y = FLOOR_CENTER_Y + FLOOR_THICKNESS * 0.5 + PLAYER_COLLIDER_RADIUS;

    let floor_bottom_face_y = FLOOR_CENTER_Y - FLOOR_THICKNESS * 0.5;

    // 첫 번째 고속 틱은 다음 이동 경로를 Broad Phase에
    // 준비시키는 틱입니다.
    //
    // 공은 y=8에서 y=4로 이동하며, 아직 y=2의 얇은 바닥에는
    // 도달하지 않아야 합니다.
    app.update();

    let primed_position = app
        .world()
        .get::<Position>(ball)
        .expect("physics must update the player position")
        .0;

    assert!(
        primed_position.y > expected_contact_y,
        "preparation tick reached the floor too early: \
     y={}, expected contact y={expected_contact_y}",
        primed_position.y
    );

    let mut lowest_y = primed_position.y;
    let mut floor_bounce = None;

    // 다음 틱의 y=4 → y=0 이동 경로가 얇은 바닥 y=2를
    // 통과하므로 Swept CCD가 관통을 막아야 합니다.
    for tick in 1..8 {
        app.update();

        let position = app
            .world()
            .get::<Position>(ball)
            .expect("physics must update the player position")
            .0;

        let velocity = app
            .world()
            .get::<LinearVelocity>(ball)
            .expect("player must have a linear velocity")
            .0;

        lowest_y = lowest_y.min(position.y);

        // 아래로 떨어지던 공의 중심이 얇은 바닥의
        // 아래쪽 면까지 넘어갔다면 관통입니다.
        assert!(
            position.y > floor_bottom_face_y,
            "player tunneled through the thin floor \
             at tick {tick}: y={}, floor bottom={floor_bottom_face_y}",
            position.y
        );

        if velocity.y >= MIN_BOUNCE_VELOCITY - VELOCITY_TOLERANCE {
            floor_bounce = Some((tick, position.y, velocity.y));

            break;
        }
    }

    let (bounce_tick, contact_y, rebound_velocity) =
        floor_bounce.expect("SweptCcd prevented no usable floor response");

    assert!(
        (contact_y - expected_contact_y).abs() <= POSITION_TOLERANCE,
        "expected high-speed floor contact y near \
         {expected_contact_y}, found {contact_y}"
    );

    assert!(
        (rebound_velocity - MIN_BOUNCE_VELOCITY).abs() <= VELOCITY_TOLERANCE,
        "expected constant floor rebound velocity near \
         {MIN_BOUNCE_VELOCITY}, \
         found {rebound_velocity}"
    );

    println!(
        "swept CCD floor result: \
         bounce tick {bounce_tick}, \
         lowest y {lowest_y:.4}, \
         contact y {contact_y:.4}, \
         rebound velocity {rebound_velocity:.4}"
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
