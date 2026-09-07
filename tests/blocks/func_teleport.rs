mod common;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        BLOCK_WORLD_SIZE, BlockPhysicsBody, GameplayPhysicsPlugin, GridIndex, MapSpawnPlugin,
        PHYSICS_HZ, PlayRestartPlugin, PlaySessionPlugin, RestartPlayWorld, SpawnValidatedMap,
        TELEPORT_SENSOR_SIZE, TeleportBlockPlugin, TeleportChannel, TeleportEntrance, TeleportExit,
    },
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use common::load_validated_map;
use std::time::Duration;

const TELEPORT_MAP: &str = include_str!("../assets/maps/phase5a_func_teleport.json");

const PLAYER_START: GridPosition = GridPosition::new(2, 3);

const TP1_IN: GridPosition = GridPosition::new(6, 3);
const TP1_OUT: GridPosition = GridPosition::new(18, 3);

const TP2_IN: GridPosition = GridPosition::new(6, 9);
const TP2_OUT: GridPosition = GridPosition::new(18, 9);

fn app_with_teleport_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        TeleportBlockPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map(TELEPORT_MAP)));

    app.update();
    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

fn entity_at(app: &App, position: GridPosition) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(position)
        .expect("expected indexed entity")
}

fn player(app: &App) -> Entity {
    entity_at(app, PLAYER_START)
}

fn world_position(position: GridPosition) -> Vec2 {
    Vec2::new(
        position.x as f32 * BLOCK_WORLD_SIZE,
        position.y as f32 * BLOCK_WORLD_SIZE,
    )
}

fn set_player_state(app: &mut App, player: Entity, position: Vec2, velocity: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_xyz(position.x, position.y, 0.0),
        LinearVelocity(velocity),
        GravityScale(0.0),
    ));
}

fn assert_vec2_close(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() <= 0.001,
        "expected {expected:?}, found {actual:?}"
    );
}

fn wait_for_teleport(app: &mut App, player: Entity, target: Vec2) {
    for _ in 0..20 {
        app.update();

        let position = app
            .world()
            .get::<Position>(player)
            .expect("player needs Position")
            .0;

        if (position - target).length() <= 0.001 {
            return;
        }
    }

    let position = app.world().get::<Position>(player).unwrap().0;

    panic!(
        "player never teleported to {target:?}; \
         final position was {position:?}"
    );
}

fn assert_entrance(app: &App, position: GridPosition, expected_channel: TeleportChannel) {
    let entity = entity_at(app, position);

    let entrance = app
        .world()
        .get::<TeleportEntrance>(entity)
        .expect("teleport entrance role missing");

    assert_eq!(entrance.channel(), expected_channel);

    assert_eq!(
        app.world().get::<RigidBody>(entity),
        Some(&RigidBody::Static),
    );

    assert!(app.world().get::<BlockPhysicsBody>(entity).is_some());
    assert!(app.world().get::<Sensor>(entity).is_some());
    assert!(app.world().get::<CollisionEventsEnabled>(entity).is_some());

    let collider = app
        .world()
        .get::<Collider>(entity)
        .expect("teleport entrance needs collider");

    let cuboid = collider
        .shape()
        .as_cuboid()
        .expect("teleport entrance collider must be rectangular");

    assert_eq!(cuboid.half_extents.x, TELEPORT_SENSOR_SIZE * 0.5,);

    assert_eq!(cuboid.half_extents.y, TELEPORT_SENSOR_SIZE * 0.5,);
}

fn assert_exit(app: &App, position: GridPosition, expected_channel: TeleportChannel) {
    let entity = entity_at(app, position);

    let exit = app
        .world()
        .get::<TeleportExit>(entity)
        .expect("teleport exit role missing");

    assert_eq!(exit.channel(), expected_channel);

    // Unity fb_tp*_out과 동일:
    // 출구는 Sprite 위치 표식일 뿐 물리 Collider가 없습니다.
    assert!(app.world().get::<RigidBody>(entity).is_none());
    assert!(app.world().get::<Collider>(entity).is_none());
    assert!(app.world().get::<Sensor>(entity).is_none());
    assert!(app.world().get::<BlockPhysicsBody>(entity).is_none());
}

fn run_teleport_case(
    entrance: GridPosition,
    exit: GridPosition,
    wrong_exit: GridPosition,
    velocity: Vec2,
) {
    let mut app = app_with_teleport_map();

    let player = player(&app);

    set_player_state(&mut app, player, world_position(entrance), velocity);

    let expected_target = world_position(exit);

    wait_for_teleport(&mut app, player, expected_target);

    let position = app.world().get::<Position>(player).unwrap().0;

    let final_velocity = app.world().get::<LinearVelocity>(player).unwrap().0;

    assert_vec2_close(position, expected_target);

    // 핵심:
    // 순간이동은 속도를 절대 바꾸지 않습니다.
    assert_vec2_close(final_velocity, velocity);

    // 다른 채널 출구로 가지 않았는지도 확인.
    assert!(
        (position - world_position(wrong_exit)).length() > 0.001,
        "player teleported to the wrong channel"
    );
}

#[test]
fn teleport_map_declares_both_channels_and_matching_exit_settings() {
    let map = load_validated_map(TELEPORT_MAP);

    assert_eq!(map.blocks.len(), 9);

    assert_eq!(map.settings.teleport_1_exit, Some(TP1_OUT),);

    assert_eq!(map.settings.teleport_2_exit, Some(TP2_OUT),);

    for (id, position) in [
        ("fb_tp1_in", TP1_IN),
        ("fb_tp1_out", TP1_OUT),
        ("fb_tp2_in", TP2_IN),
        ("fb_tp2_out", TP2_OUT),
    ] {
        let block = map.block_at(position).expect("teleport block missing");

        assert_eq!(block.id.as_str(), id);
    }
}

#[test]
fn teleport_inlets_are_sensors_but_outlets_have_no_physics_body() {
    let app = app_with_teleport_map();

    assert_entrance(&app, TP1_IN, TeleportChannel::One);

    assert_exit(&app, TP1_OUT, TeleportChannel::One);

    assert_entrance(&app, TP2_IN, TeleportChannel::Two);

    assert_exit(&app, TP2_OUT, TeleportChannel::Two);
}

#[test]
fn both_channels_teleport_to_their_own_exit_and_preserve_velocity() {
    run_teleport_case(TP1_IN, TP1_OUT, TP2_OUT, Vec2::new(4.0, -7.0));

    run_teleport_case(TP2_IN, TP2_OUT, TP1_OUT, Vec2::new(-3.0, 5.0));
}

#[test]
fn entrance_does_nothing_when_its_matching_exit_is_unavailable() {
    let mut app = app_with_teleport_map();

    let tp1_out = entity_at(&app, TP1_OUT);

    // ValidatedMap 자체는 정상 상태로 생성한 뒤,
    // 런타임에서 출구 역할만 제거하여
    // Unity의 isOpen == false 상황을 재현합니다.
    app.world_mut().entity_mut(tp1_out).remove::<TeleportExit>();

    let player = player(&app);

    let entrance_position = world_position(TP1_IN);

    set_player_state(&mut app, player, entrance_position, Vec2::ZERO);

    for _ in 0..5 {
        app.update();
    }

    let position = app.world().get::<Position>(player).unwrap().0;

    assert_vec2_close(position, entrance_position);
}

#[test]
fn restart_recreates_teleports_and_they_still_work_after_restart() {
    let mut app = app_with_teleport_map();

    let old_player = player(&app);
    let old_tp1_in = entity_at(&app, TP1_IN);
    let old_tp1_out = entity_at(&app, TP1_OUT);

    // Restart 전 실제 TP1 동작 확인.
    set_player_state(&mut app, old_player, world_position(TP1_IN), Vec2::ZERO);

    wait_for_teleport(&mut app, old_player, world_position(TP1_OUT));

    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    assert!(!app.world().entities().contains(old_player));
    assert!(!app.world().entities().contains(old_tp1_in));
    assert!(!app.world().entities().contains(old_tp1_out));

    let new_player = player(&app);
    let new_tp1_in = entity_at(&app, TP1_IN);
    let new_tp1_out = entity_at(&app, TP1_OUT);

    assert_ne!(new_player, old_player);
    assert_ne!(new_tp1_in, old_tp1_in);
    assert_ne!(new_tp1_out, old_tp1_out);

    assert_entrance(&app, TP1_IN, TeleportChannel::One);

    assert_exit(&app, TP1_OUT, TeleportChannel::One);

    // Restart 후에는 TP2까지 실제 동작 확인.
    set_player_state(
        &mut app,
        new_player,
        world_position(TP2_IN),
        Vec2::new(2.0, 0.0),
    );

    wait_for_teleport(&mut app, new_player, world_position(TP2_OUT));

    assert_vec2_close(
        app.world().get::<LinearVelocity>(new_player).unwrap().0,
        Vec2::new(2.0, 0.0),
    );
}
