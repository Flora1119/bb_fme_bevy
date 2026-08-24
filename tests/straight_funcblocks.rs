use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        BlockPhysicsBody, ConsumedFunctionBlock, GameplayPhysicsPlugin, GridIndex, MapSpawnPlugin,
        OneShotFunctionBlock, PHYSICS_HZ, PLAYER_GRAVITY_SCALE, PlaySessionPlugin, PlayerBall,
        PlayerControlPlugin, SolidBlock, SpawnValidatedMap, StraightBlock, StraightBrake,
        StraightMovement,
    },
    map::MapDocument,
};
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const STRAIGHT_MAP: &str = r#"
{
    "map_name": "straight_funcblocks",
    "author": "Development",
    "map_settings": {
        "time_limit": 60.0,
        "show_time_ranking": false,
        "star_count": 1,
        "size": {
            "width": 25,
            "height": 15
        },
        "tp1_exit": {
            "x": -1,
            "y": -1
        },
        "tp2_exit": {
            "x": -1,
            "y": -1
        },
        "portal1_positions": {
            "a_px": -1,
            "a_py": -1,
            "b_px": -1,
            "b_py": -1
        },
        "portal2_positions": {
            "a_px": -1,
            "a_py": -1,
            "b_px": -1,
            "b_py": -1
        },
        "sw_el": true,
        "sw_b1": true,
        "sw_b2": true
    },
    "blocks": [
        {
            "x": 2,
            "y": 3,
            "block": {
                "type": "item",
                "name": "ball",
                "dir": 0
            }
        },
        {
            "x": 23,
            "y": 13,
            "block": {
                "type": "item",
                "name": "star",
                "dir": 0
            }
        },
        {
            "x": 5,
            "y": 0,
            "block": {
                "type": "funcblock",
                "name": "fb_st_hv",
                "dir": 1
            }
        },
        {
            "x": 9,
            "y": 0,
            "block": {
                "type": "funcblock",
                "name": "fb_st_dg",
                "dir": 0
            }
        },
        {
            "x": 13,
            "y": 0,
            "block": {
                "type": "funcblock",
                "name": "fb_ds_st_hv",
                "dir": 1
            }
        },
        {
            "x": 17,
            "y": 0,
            "block": {
                "type": "funcblock",
                "name": "fb_ds_st_dg",
                "dir": 0
            }
        }
    ],
    "block_options": []
}
"#;

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(STRAIGHT_MAP).expect("straight map must deserialize");

    ValidatedMap::from_document(&document, &config).expect("straight map must validate")
}

fn app_with_straight_blocks() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayerControlPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

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
    app.world_mut()
        .entity_mut(player)
        .remove::<StraightMovement>();

    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity::ZERO,
        GravityScale(PLAYER_GRAVITY_SCALE),
    ));
}

fn activate_straight_block(app: &mut App, block_x: f32) -> Entity {
    let player = player(app);

    set_player_state(app, player, Vec2::new(block_x, 2.0));

    for _ in 0..80 {
        app.update();

        if app.world().get::<StraightMovement>(player).is_some() {
            return player;
        }
    }

    panic!(
        "player never entered \
         StraightMovement"
    );
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.001,
        "expected {expected}, \
         found {actual}"
    );
}

fn assert_vec2_close(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() <= 0.001,
        "expected {expected:?}, \
         found {actual:?}"
    );
}

#[test]
fn straight_funcblocks_have_the_expected_runtime_roles() {
    let app = app_with_straight_blocks();

    let cases = [
        (5, Vec2::new(1.0, 0.0), StraightBlock::STANDARD_SPEED, false),
        (9, Vec2::new(1.0, 1.0), StraightBlock::STANDARD_SPEED, false),
        (13, Vec2::new(1.0, 0.0), StraightBlock::HIGH_SPEED, true),
        (17, Vec2::new(1.0, 1.0), StraightBlock::HIGH_SPEED, true),
    ];

    for (x, expected_offset, expected_speed, one_shot) in cases {
        let entity = entity_at(&app, x, 0);

        assert!(app.world().get::<SolidBlock>(entity,).is_some());

        assert_eq!(
            app.world().get::<RigidBody>(entity,),
            Some(&RigidBody::Static)
        );

        let straight = *app.world().get::<StraightBlock>(entity).expect(
            "funcblock must have \
                 StraightBlock",
        );

        assert_vec2_close(straight.exit_offset(), expected_offset);

        assert_close(straight.speed(), expected_speed);

        assert_eq!(
            app.world().get::<OneShotFunctionBlock>(entity,).is_some(),
            one_shot
        );
    }
}

#[test]
fn cardinal_straight_launch_repositions_the_player_and_disables_gravity() {
    let mut app = app_with_straight_blocks();

    let player = activate_straight_block(&mut app, 5.0);

    let movement = *app
        .world()
        .get::<StraightMovement>(player)
        .expect("player must be straight moving");

    assert_vec2_close(movement.direction(), Vec2::X);

    assert_close(movement.speed(), StraightBlock::STANDARD_SPEED);

    assert_vec2_close(
        app.world().get::<Position>(player).unwrap().0,
        Vec2::new(6.0, 0.0),
    );

    assert_vec2_close(
        app.world().get::<LinearVelocity>(player).unwrap().0,
        Vec2::new(StraightBlock::STANDARD_SPEED, 0.0),
    );

    assert_close(app.world().get::<GravityScale>(player).unwrap().0, 0.0);

    // 입력이 없는 상태에서도 일반 제어의
    // 감속 로직이 StraightMovement를
    // 건드리면 안 됩니다.
    //
    // 너무 오래 진행하면 x=9의 다음
    // 테스트 블록과 충돌하므로 1틱만
    // 확인합니다.
    app.update();

    assert_vec2_close(
        app.world().get::<LinearVelocity>(player).unwrap().0,
        Vec2::new(StraightBlock::STANDARD_SPEED, 0.0),
    );

    // 직진 중 새 입력 = 직진 취소.
    //
    // ButtonInput을 직접 press()하면
    // Bevy InputSystems가 PreUpdate에서
    // just_pressed 상태를 초기화하므로,
    // 실제 KeyboardInput 메시지를 주입합니다.
    let test_window = app.world_mut().spawn_empty().id();

    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::ArrowLeft,
        logical_key: Key::ArrowLeft,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: test_window,
    });

    app.update();

    assert!(app.world().get::<StraightMovement>(player,).is_none());

    assert_close(
        app.world().get::<GravityScale>(player).unwrap().0,
        PLAYER_GRAVITY_SCALE,
    );

    // 직진은 즉시 해제되지만,
    // 속도 자체가 순간적으로 0이 되면
    // 안 됩니다.
    assert!(app.world().get::<StraightBrake>(player,).is_some());

    let speed_after_press = app.world().get::<LinearVelocity>(player).unwrap().0.x;

    assert!(
        speed_after_press > 0.0,
        "straight cancel stopped \
     the player instantly"
    );

    assert!(
        speed_after_press < StraightBlock::STANDARD_SPEED,
        "straight brake did not \
     reduce forward speed"
    );

    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::ArrowLeft,
        logical_key: Key::ArrowLeft,
        state: ButtonState::Released,
        text: None,
        repeat: false,
        window: test_window,
    });

    let mut brake_finished = false;

    for _ in 0..20 {
        app.update();

        if app.world().get::<StraightBrake>(player).is_none() {
            brake_finished = true;
            break;
        }
    }

    assert!(
        brake_finished,
        "straight brake did not \
     finish quickly enough"
    );

    let final_velocity = app.world().get::<LinearVelocity>(player).unwrap().0;

    // 오른쪽으로 가던 직진 속도는
    // 거의 완전히 제거돼야 합니다.
    // Y는 중력 때문에 내려갈 수 있으므로
    // X만 검사합니다.
    assert!(
        final_velocity.x.abs() <= 0.11,
        "straight brake left too much \
     forward speed: {}",
        final_velocity.x
    );
}

#[test]
fn diagonal_straight_launch_uses_normalized_speed() {
    let mut app = app_with_straight_blocks();

    let player = activate_straight_block(&mut app, 9.0);

    let velocity = app.world().get::<LinearVelocity>(player).unwrap().0;

    assert_close(velocity.length(), StraightBlock::STANDARD_SPEED);

    assert_vec2_close(velocity.normalize(), Vec2::new(1.0, 1.0).normalize());

    assert_vec2_close(
        app.world().get::<Position>(player).unwrap().0,
        Vec2::new(10.0, 1.0),
    );
}

#[test]
fn high_straight_block_launches_at_15_and_is_consumed() {
    let mut app = app_with_straight_blocks();

    let source = entity_at(&app, 13, 0);

    let player = activate_straight_block(&mut app, 13.0);

    let velocity = app.world().get::<LinearVelocity>(player).unwrap().0;

    assert_close(velocity.length(), StraightBlock::HIGH_SPEED);

    assert!(app.world().get::<ConsumedFunctionBlock>(source,).is_some());

    assert!(app.world().get::<ColliderDisabled>(source,).is_some());

    assert!(matches!(
        app.world().get::<Visibility>(source,),
        Some(Visibility::Hidden)
    ));
}

#[test]
fn hitting_a_wall_stops_straight_movement_and_restores_gravity() {
    let mut app = app_with_straight_blocks();

    let player = activate_straight_block(&mut app, 5.0);

    app.world_mut().spawn((
        Name::new("Straight movement test wall"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(1.0, 3.0),
        Transform::from_xyz(8.0, 0.0, 0.0),
    ));

    let mut stopped = false;

    for _ in 0..40 {
        app.update();

        if app.world().get::<StraightMovement>(player).is_none() {
            stopped = true;
            break;
        }
    }

    assert!(
        stopped,
        "wall collision did not stop \
         StraightMovement"
    );

    assert_close(
        app.world().get::<GravityScale>(player).unwrap().0,
        PLAYER_GRAVITY_SCALE,
    );

    assert!(
        app.world().get::<LinearVelocity>(player,).unwrap().0.x < 0.0,
        "wall collision should bounce \
         the player away from the wall"
    );
}
