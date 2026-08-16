use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        BlockPhysicsBody, GameplayPhysicsPlugin, GridIndex, MIN_WALL_BOUNCE_SPEED, MapSpawnPlugin,
        PHYSICS_HZ, PLAYER_HORIZONTAL_ACCELERATION, PLAYER_HORIZONTAL_DECELERATION,
        PLAYER_MAX_HORIZONTAL_SPEED, PlayerControlPlugin, PlayerInputIntent, SOLID_COLLIDER_SIZE,
        SolidBlock, SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");
const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

const PLAYER_START_X: f32 = 2.0;
const WALL_DISTANCE_FROM_PLAYER: f32 = 1.0;
const WALL_LAUNCH_SPEED: f32 = 5.0;
const OBSERVATION_TICKS: usize = 10;
const VELOCITY_TOLERANCE: f32 = 0.06;

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(MINIMAL_MAP).expect("map fixture must deserialize");

    ValidatedMap::from_document(&document, &config).expect("fixture must validate")
}

fn app_with_player_control() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlayerControlPlugin,
    ));
    app.init_resource::<Assets<GizmoAsset>>();
    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    app.finish();
    app.cleanup();

    // 첫 Update에서 맵, 물리 본체, PlayerInputIntent를 구성합니다.
    app.update();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

fn player_ball(app: &App) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("player ball must be indexed")
}

fn set_horizontal_input(app: &mut App, horizontal: i8) {
    assert!((-1..=1).contains(&horizontal));

    let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();

    keyboard.release_all();

    match horizontal {
        -1 => keyboard.press(KeyCode::ArrowLeft),
        0 => {}
        1 => keyboard.press(KeyCode::ArrowRight),
        _ => unreachable!(),
    }
}

fn horizontal_velocity(app: &App, player: Entity) -> f32 {
    app.world()
        .get::<LinearVelocity>(player)
        .expect("player must have a linear velocity")
        .0
        .x
}

fn assert_velocity_close(actual: f32, expected: f32, tick: usize) {
    assert!(
        (actual - expected).abs() <= VELOCITY_TOLERANCE,
        "tick {tick}: expected horizontal velocity near {expected}, found {actual}"
    );
}

fn launch_into_wall_and_capture_rebound(
    app: &mut App,
    player: Entity,
    launch_direction: f32,
) -> f32 {
    assert!(launch_direction == -1.0 || launch_direction == 1.0);
    assert!(
        app.world().get::<PlayerInputIntent>(player).is_some(),
        "PlayerControlPlugin must attach PlayerInputIntent"
    );

    // 수평 조작과 벽 반동의 합성만 검증하기 위해 중력을 끕니다.
    *app.world_mut()
        .get_mut::<GravityScale>(player)
        .expect("player must have a gravity scale") = GravityScale(0.0);

    let wall_center_x = PLAYER_START_X + WALL_DISTANCE_FROM_PLAYER * launch_direction;

    app.world_mut().spawn((
        Name::new("Test wall for player control"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
        Transform::from_xyz(wall_center_x, 2.0, 0.0),
    ));

    set_horizontal_input(app, launch_direction as i8);

    app.world_mut()
        .get_mut::<LinearVelocity>(player)
        .expect("player must have a linear velocity")
        .0 = Vec2::new(WALL_LAUNCH_SPEED * launch_direction, 0.0);

    let expected_rebound = -MIN_WALL_BOUNCE_SPEED * launch_direction;

    for _ in 0..20 {
        app.update();

        let velocity = horizontal_velocity(app, player);

        if velocity * expected_rebound.signum() > 1.0 {
            assert!(
                (velocity - expected_rebound).abs() <= VELOCITY_TOLERANCE,
                "expected wall rebound velocity near {expected_rebound}, found {velocity}"
            );

            return velocity;
        }
    }

    panic!("player did not bounce from the test wall");
}

#[test]
fn input_in_the_rebound_direction_accelerates_toward_control_speed() {
    let velocity_step = PLAYER_HORIZONTAL_ACCELERATION / PHYSICS_HZ as f32;

    for launch_direction in [-1.0, 1.0] {
        let mut app = app_with_player_control();
        let player = player_ball(&app);
        let rebound_velocity =
            launch_into_wall_and_capture_rebound(&mut app, player, launch_direction);

        // 벽 반동 3.0은 일반 제어 목표 속도 5.0보다 느립니다.
        // 반동 방향을 계속 입력하면 매 틱 0.4씩 가속하고,
        // 일반 제어 목표 속도에서 멈춥니다.
        set_horizontal_input(&mut app, -launch_direction as i8);

        for tick in 1..=OBSERVATION_TICKS {
            app.update();

            let expected_speed = (rebound_velocity.abs() + velocity_step * tick as f32)
                .min(PLAYER_MAX_HORIZONTAL_SPEED);

            let expected = rebound_velocity.signum() * expected_speed;

            assert_velocity_close(horizontal_velocity(&app, player), expected, tick);
        }
    }
}

#[test]
fn opposite_input_reclaims_wall_rebound_speed_at_the_acceleration_rate() {
    let velocity_step = PLAYER_HORIZONTAL_ACCELERATION / PHYSICS_HZ as f32;

    for launch_direction in [-1.0, 1.0] {
        let mut app = app_with_player_control();
        let player = player_ball(&app);
        let rebound_velocity =
            launch_into_wall_and_capture_rebound(&mut app, player, launch_direction);

        // 튕기는 동안 다시 벽 쪽을 입력하면 매 물리 틱 0.4씩
        // 속도를 되찾되, 반동을 즉시 일반 최대 속도로 잘라서는 안 됩니다.
        set_horizontal_input(&mut app, launch_direction as i8);

        for tick in 1..=OBSERVATION_TICKS {
            app.update();

            let expected = rebound_velocity + velocity_step * launch_direction * tick as f32;
            assert_velocity_close(horizontal_velocity(&app, player), expected, tick);
        }
    }
}

#[test]
fn neutral_input_decelerates_wall_rebound_at_the_release_rate() {
    let velocity_step = PLAYER_HORIZONTAL_DECELERATION / PHYSICS_HZ as f32;

    for launch_direction in [-1.0, 1.0] {
        let mut app = app_with_player_control();
        let player = player_ball(&app);
        let rebound_velocity =
            launch_into_wall_and_capture_rebound(&mut app, player, launch_direction);

        // 입력을 놓으면 매 물리 틱 0.16씩 0을 향해 감속합니다.
        set_horizontal_input(&mut app, 0);

        for tick in 1..=OBSERVATION_TICKS {
            app.update();

            let expected = rebound_velocity + velocity_step * launch_direction * tick as f32;
            assert_velocity_close(horizontal_velocity(&app, player), expected, tick);
        }
    }
}
