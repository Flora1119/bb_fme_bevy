mod common;

use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::{CardinalDirection, GridPosition},
    gameplay::{
        BlockFacing, BlockIdentity, ConsumedFunctionBlock, GameplayPhysicsPlugin, GridIndex,
        MapSpawnPlugin, OneShotFunctionBlock, PHYSICS_HZ, PLAYER_GRAVITY_SCALE, PlayRestartPlugin,
        PlaySessionPlugin, PlayerControlPlugin, RestartPlayWorld, SpawnValidatedMap, StraightBlock,
        StraightMovement,
    },
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use common::load_validated_map;
use std::time::Duration;

const STRAIGHT_MAP: &str = include_str!("../assets/maps/phase5a_func_straight.json");

const CASES: [(&str, i32, CardinalDirection, Vec2, f32, bool); 16] = [
    (
        "fb_st_hv",
        2,
        CardinalDirection::Up,
        Vec2::new(0.0, 1.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_st_hv",
        2,
        CardinalDirection::Right,
        Vec2::new(1.0, 0.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_st_hv",
        2,
        CardinalDirection::Down,
        Vec2::new(0.0, -1.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_st_hv",
        2,
        CardinalDirection::Left,
        Vec2::new(-1.0, 0.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_st_dg",
        5,
        CardinalDirection::Up,
        Vec2::new(1.0, 1.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_st_dg",
        5,
        CardinalDirection::Right,
        Vec2::new(1.0, -1.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_st_dg",
        5,
        CardinalDirection::Down,
        Vec2::new(-1.0, -1.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_st_dg",
        5,
        CardinalDirection::Left,
        Vec2::new(-1.0, 1.0),
        StraightBlock::STANDARD_SPEED,
        false,
    ),
    (
        "fb_ds_st_hv",
        8,
        CardinalDirection::Up,
        Vec2::new(0.0, 1.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
    (
        "fb_ds_st_hv",
        8,
        CardinalDirection::Right,
        Vec2::new(1.0, 0.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
    (
        "fb_ds_st_hv",
        8,
        CardinalDirection::Down,
        Vec2::new(0.0, -1.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
    (
        "fb_ds_st_hv",
        8,
        CardinalDirection::Left,
        Vec2::new(-1.0, 0.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
    (
        "fb_ds_st_dg",
        11,
        CardinalDirection::Up,
        Vec2::new(1.0, 1.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
    (
        "fb_ds_st_dg",
        11,
        CardinalDirection::Right,
        Vec2::new(1.0, -1.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
    (
        "fb_ds_st_dg",
        11,
        CardinalDirection::Down,
        Vec2::new(-1.0, -1.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
    (
        "fb_ds_st_dg",
        11,
        CardinalDirection::Left,
        Vec2::new(-1.0, 1.0),
        StraightBlock::HIGH_SPEED,
        true,
    ),
];

const X_BY_DIRECTION: [i32; 4] = [6, 10, 14, 18];

fn app_with_straight_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        PlayerControlPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map(STRAIGHT_MAP)));

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

fn direction_x(direction: CardinalDirection) -> i32 {
    X_BY_DIRECTION[direction.index() as usize]
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

fn set_player_above(app: &mut App, player: Entity, x: f32, y: f32) {
    app.world_mut()
        .entity_mut(player)
        .remove::<StraightMovement>();

    app.world_mut().entity_mut(player).insert((
        Position(Vec2::new(x, y + 2.0)),
        Transform::from_xyz(x, y + 2.0, 0.0),
        LinearVelocity::ZERO,
        GravityScale(PLAYER_GRAVITY_SCALE),
    ));
}

fn wait_for_straight_launch(app: &mut App, player: Entity) {
    for _ in 0..100 {
        app.update();

        if app.world().get::<StraightMovement>(player).is_some() {
            return;
        }
    }

    panic!(
        "player never entered \
         StraightMovement"
    );
}

#[test]
fn development_map_contains_all_four_straight_profiles_in_all_directions() {
    let map = load_validated_map(STRAIGHT_MAP);

    assert_eq!(map.blocks.len(), 21);

    for id in ["fb_st_hv", "fb_st_dg", "fb_ds_st_hv", "fb_ds_st_dg"] {
        assert_eq!(
            map.blocks
                .iter()
                .filter(|block| { block.id.as_str() == id })
                .count(),
            4,
            "{id} must appear four times"
        );

        for direction in CardinalDirection::ALL {
            assert!(
                map.blocks
                    .iter()
                    .any(|block| { block.id.as_str() == id && block.direction == direction },),
                "{id} is missing \
                 {direction:?}"
            );
        }
    }
}

#[test]
fn all_sixteen_straight_blocks_have_correct_runtime_profiles() {
    let app = app_with_straight_map();

    for (id, y, direction, expected_offset, expected_speed, one_shot) in CASES {
        let x = direction_x(direction);

        let entity = entity_at(&app, x, y);

        let identity = app
            .world()
            .get::<BlockIdentity>(entity)
            .expect("straight block needs identity");

        let facing = app
            .world()
            .get::<BlockFacing>(entity)
            .expect("straight block needs facing");

        let straight = *app.world().get::<StraightBlock>(entity).expect(
            "straight block needs \
                 StraightBlock",
        );

        assert_eq!(identity.id.as_str(), id);

        assert_eq!(facing.0, direction);

        assert_vec2_close(straight.exit_offset(), expected_offset);

        assert_close(straight.speed(), expected_speed);

        assert_eq!(
            app.world().get::<OneShotFunctionBlock>(entity,).is_some(),
            one_shot,
            "{id} one-shot role mismatch"
        );

        assert_eq!(
            app.world().get::<RigidBody>(entity,),
            Some(&RigidBody::Static)
        );

        assert!(app.world().get::<Collider>(entity,).is_some());
    }
}

#[test]
fn all_sixteen_blocks_launch_in_the_expected_direction_and_speed() {
    for (id, y, direction, expected_offset, expected_speed, one_shot) in CASES {
        // 케이스마다 새 App을 만들어
        // 이전 직진/충돌/소멸 상태가
        // 다음 케이스에 영향을 주지 않게 합니다.
        let mut app = app_with_straight_map();

        let x = direction_x(direction);

        let source = entity_at(&app, x, y);

        let player = player(&app);

        set_player_above(&mut app, player, x as f32, y as f32);

        wait_for_straight_launch(&mut app, player);

        let movement = *app.world().get::<StraightMovement>(player).expect(
            "launch must create \
                 StraightMovement",
        );

        let position = app.world().get::<Position>(player).unwrap().0;

        let velocity = app.world().get::<LinearVelocity>(player).unwrap().0;

        let expected_direction = expected_offset.normalize();

        let expected_position = Vec2::new(x as f32, y as f32) + expected_offset;

        assert_vec2_close(movement.direction(), expected_direction);

        assert_close(movement.speed(), expected_speed);

        assert_vec2_close(position, expected_position);

        assert_vec2_close(velocity, expected_direction * expected_speed);

        assert_close(app.world().get::<GravityScale>(player).unwrap().0, 0.0);

        assert_eq!(
            app.world().get::<ConsumedFunctionBlock>(source,).is_some(),
            one_shot,
            "{id} {direction:?} \
             consumption mismatch"
        );

        assert_eq!(
            app.world().get::<ColliderDisabled>(source,).is_some(),
            one_shot
        );
    }
}

#[test]
fn restart_restores_all_consumed_high_speed_straight_blocks() {
    let mut app = app_with_straight_map();

    // 대표로 고속 HV 하나와
    // 고속 DG 하나를 소모시킵니다.
    let old_hv = entity_at(&app, 10, 8);

    let old_dg = entity_at(&app, 18, 11);

    let player_entity = player(&app);

    set_player_above(&mut app, player_entity, 10.0, 8.0);

    wait_for_straight_launch(&mut app, player_entity);

    assert!(app.world().get::<ConsumedFunctionBlock>(old_hv,).is_some());

    set_player_above(&mut app, player_entity, 18.0, 11.0);

    wait_for_straight_launch(&mut app, player_entity);

    assert!(app.world().get::<ConsumedFunctionBlock>(old_dg,).is_some());

    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    assert!(!app.world().entities().contains(old_hv));

    assert!(!app.world().entities().contains(old_dg));

    let new_hv = entity_at(&app, 10, 8);

    let new_dg = entity_at(&app, 18, 11);

    assert_ne!(new_hv, old_hv);

    assert_ne!(new_dg, old_dg);

    for entity in [new_hv, new_dg] {
        assert!(app.world().get::<OneShotFunctionBlock>(entity,).is_some());

        assert!(app.world().get::<ConsumedFunctionBlock>(entity,).is_none());

        assert!(app.world().get::<ColliderDisabled>(entity,).is_none());

        assert!(app.world().get::<Collider>(entity,).is_some());
    }

    let mut straight_query = app.world_mut().query::<&StraightBlock>();

    assert_eq!(straight_query.iter(app.world()).count(), 16);

    let mut one_shot_query = app.world_mut().query::<&OneShotFunctionBlock>();

    assert_eq!(one_shot_query.iter(app.world()).count(), 8);
}
