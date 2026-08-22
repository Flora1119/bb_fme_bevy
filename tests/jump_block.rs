use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        BlockPhysicsBody, GameplayPhysicsPlugin, GridIndex, JumpBlock, MapSpawnPlugin, PHYSICS_HZ,
        PLAYER_COLLIDER_RADIUS, PlaySessionPlugin, SolidBlock, SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const JUMP_MAP: &str = include_str!("../assets/maps/phase4_jump_boundary_sandbox.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument = serde_json::from_str(JUMP_MAP).expect("jump map must deserialize");

    ValidatedMap::from_document(&document, &config).expect("jump map must validate")
}

fn app_with_jump_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
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

fn player(app: &App) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(4, 2))
        .expect("player must be indexed")
}

fn set_player_state(app: &mut App, entity: Entity, position: Vec2, velocity: Vec2) {
    app.world_mut().entity_mut(entity).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(velocity),
    ));
}

#[test]
fn fb_jump_is_both_solid_and_jump_block() {
    let app = app_with_jump_map();

    let jump = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(6, 0))
        .expect("fb_jump must be indexed");

    assert!(app.world().get::<SolidBlock>(jump).is_some());

    let jump_block = app
        .world()
        .get::<JumpBlock>(jump)
        .expect("fb_jump must have JumpBlock");

    assert_eq!(jump_block.launch_speed(), JumpBlock::STANDARD_LAUNCH_SPEED);

    assert_eq!(app.world().get::<RigidBody>(jump), Some(&RigidBody::Static));

    assert!(app.world().get::<Collider>(jump).is_some());
}

#[test]
fn jump_block_launches_opposite_downward_gravity() {
    let mut app = app_with_jump_map();

    let ball = player(&app);

    set_player_state(&mut app, ball, Vec2::new(6.0, 2.0), Vec2::ZERO);

    let mut strongest_upward_speed = f32::NEG_INFINITY;

    for _ in 0..100 {
        app.update();

        let speed = app.world().get::<LinearVelocity>(ball).unwrap().0.y;

        strongest_upward_speed = strongest_upward_speed.max(speed);
    }

    assert!(
        strongest_upward_speed >= JumpBlock::STANDARD_LAUNCH_SPEED - 0.7,
        "jump block did not produce the expected \
         upward launch: {strongest_upward_speed}"
    );

    assert!(
        strongest_upward_speed <= JumpBlock::STANDARD_LAUNCH_SPEED + 0.1,
        "jump speed was unexpectedly amplified: \
         {strongest_upward_speed}"
    );
}

#[test]
fn jump_block_launches_opposite_upward_gravity() {
    let mut app = app_with_jump_map();

    app.world_mut().resource_mut::<Gravity>().0 = Vec2::new(0.0, 9.81);

    let ball = player(&app);

    set_player_state(&mut app, ball, Vec2::new(6.0, -2.0), Vec2::ZERO);

    let mut strongest_downward_speed = f32::INFINITY;

    for _ in 0..100 {
        app.update();

        let speed = app.world().get::<LinearVelocity>(ball).unwrap().0.y;

        strongest_downward_speed = strongest_downward_speed.min(speed);
    }

    assert!(
        strongest_downward_speed <= -JumpBlock::STANDARD_LAUNCH_SPEED + 0.7,
        "jump block did not launch against upward \
         gravity: {strongest_downward_speed}"
    );

    assert!(
        strongest_downward_speed >= -JumpBlock::STANDARD_LAUNCH_SPEED - 0.1,
        "jump speed was unexpectedly amplified: \
         {strongest_downward_speed}"
    );
}

#[test]
fn touching_two_jump_blocks_at_a_corner_does_not_stack_speed() {
    let mut app = app_with_jump_map();

    // x=5의 일반 블록도 이 테스트에서만 JumpBlock으로 만듭니다.
    let second_jump = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(5, 0))
        .expect("second floor block must exist");

    app.world_mut()
        .entity_mut(second_jump)
        .insert(JumpBlock::standard());

    let ball = player(&app);

    // x=5와 x=6 블록 사이의 경계에 정확히 떨어뜨립니다.
    set_player_state(&mut app, ball, Vec2::new(5.5, 2.0), Vec2::ZERO);

    let mut strongest_speed = f32::NEG_INFINITY;

    for _ in 0..100 {
        app.update();

        let speed = app.world().get::<LinearVelocity>(ball).unwrap().0.y;

        strongest_speed = strongest_speed.max(speed);
    }

    assert!(
        strongest_speed >= JumpBlock::STANDARD_LAUNCH_SPEED - 0.7,
        "corner contact never produced a jump"
    );

    assert!(
        strongest_speed <= JumpBlock::STANDARD_LAUNCH_SPEED + 0.1,
        "corner contact stacked jump speed: \
         {strongest_speed}"
    );
}

#[test]
fn swept_ccd_keeps_jump_launch_from_crossing_a_thin_ceiling() {
    const CEILING_Y: f32 = 2.0;
    const CEILING_THICKNESS: f32 = 0.05;
    const CEILING_WIDTH: f32 = 1.0;
    const POSITION_TOLERANCE: f32 = 0.1;

    let mut app = app_with_jump_map();

    let ball = player(&app);

    assert!(
        app.world().get::<SweptCcd>(ball).is_some(),
        "player must keep SweptCcd enabled"
    );

    // Speculative Collision을 끄고
    // Swept CCD가 있는 조건에서 재검증합니다.
    app.world_mut()
        .resource_mut::<NarrowPhaseConfig>()
        .default_speculative_margin = 0.0;

    set_player_state(&mut app, ball, Vec2::new(6.0, 1.2), Vec2::ZERO);

    app.world_mut().spawn((
        Name::new("Test thin jump ceiling"),
        SolidBlock,
        BlockPhysicsBody,
        RigidBody::Static,
        Collider::rectangle(CEILING_WIDTH, CEILING_THICKNESS),
        Transform::from_xyz(6.0, CEILING_Y, 0.0),
    ));

    let expected_contact_y = CEILING_Y - CEILING_THICKNESS * 0.5 - PLAYER_COLLIDER_RADIUS;

    let mut launched = false;
    let mut stopped_at_ceiling = false;
    let mut highest_y = f32::NEG_INFINITY;

    for _ in 0..120 {
        app.update();

        let position = app.world().get::<Position>(ball).unwrap().0;

        let velocity = app.world().get::<LinearVelocity>(ball).unwrap().0;

        highest_y = highest_y.max(position.y);

        if velocity.y > 10.0 {
            launched = true;
        }

        if launched && velocity.y <= 0.1 {
            stopped_at_ceiling = true;
            break;
        }
    }

    assert!(launched, "player never received the jump launch");

    assert!(
        stopped_at_ceiling,
        "player never collided with the thin ceiling"
    );

    assert!(
        highest_y <= expected_contact_y + POSITION_TOLERANCE,
        "player crossed the thin ceiling: \
         highest y={highest_y}, \
         expected contact={expected_contact_y}"
    );
}
