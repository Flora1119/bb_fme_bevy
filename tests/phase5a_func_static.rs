use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        ConsumedFunctionBlock, GameplayPhysicsPlugin, GridIndex, JumpBlock, MapSpawnPlugin,
        OneShotFunctionBlock, PHYSICS_HZ, PlayRestartPlugin, PlaySessionPlugin, RestartPlayWorld,
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

const FUNC_MAP: &str = include_str!("../assets/maps/phase5a_func_static.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument = serde_json::from_str(FUNC_MAP).expect("func map must deserialize");

    ValidatedMap::from_document(&document, &config).expect("func map must validate")
}

fn app_with_func_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
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
        .expect("expected indexed block")
}

fn player(app: &App) -> Entity {
    entity_at(app, 3, 2)
}

fn set_player_state(app: &mut App, player: Entity, position: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity::ZERO,
    ));
}

fn strongest_upward_speed(app: &mut App, player: Entity, ticks: usize) -> f32 {
    let mut strongest = f32::NEG_INFINITY;

    for _ in 0..ticks {
        app.update();

        let speed = app
            .world()
            .get::<LinearVelocity>(player)
            .expect("player must have velocity")
            .0
            .y;

        strongest = strongest.max(speed);
    }

    strongest
}

#[test]
fn jump_funcblocks_match_the_unity_runtime_roles() {
    let app = app_with_func_map();

    let standard = entity_at(&app, 5, 0);

    let high = entity_at(&app, 9, 0);

    for entity in [standard, high] {
        assert!(app.world().get::<SolidBlock>(entity,).is_some());

        assert_eq!(
            app.world().get::<RigidBody>(entity,),
            Some(&RigidBody::Static)
        );

        let collider = app.world().get::<Collider>(entity).expect(
            "jump funcblock must \
                 have a collider",
        );

        let cuboid = collider.shape().as_cuboid().expect(
            "jump funcblock collider \
                 must be rectangular",
        );

        assert_eq!(cuboid.half_extents, Vec2::splat(0.5));
    }

    assert_eq!(
        app.world()
            .get::<JumpBlock>(standard)
            .expect(
                "fb_jump must have \
                 JumpBlock",
            )
            .launch_speed(),
        JumpBlock::STANDARD_LAUNCH_SPEED
    );

    assert!(app.world().get::<OneShotFunctionBlock>(standard,).is_none());

    assert_eq!(
        app.world()
            .get::<JumpBlock>(high)
            .expect(
                "fb_ds_jump must have \
                 JumpBlock",
            )
            .launch_speed(),
        JumpBlock::HIGH_LAUNCH_SPEED
    );

    assert!(app.world().get::<OneShotFunctionBlock>(high,).is_some());
}

#[test]
fn standard_jump_block_launches_repeatedly() {
    let mut app = app_with_func_map();

    let player = player(&app);
    let jump = entity_at(&app, 5, 0);

    set_player_state(&mut app, player, Vec2::new(5.0, 2.0));

    let first = strongest_upward_speed(&mut app, player, 100);

    assert!(
        first >= JumpBlock::STANDARD_LAUNCH_SPEED - 0.7,
        "first fb_jump launch too weak: \
         {first}"
    );

    assert!(app.world().get::<ConsumedFunctionBlock>(jump,).is_none());

    assert!(app.world().get::<ColliderDisabled>(jump,).is_none());

    set_player_state(&mut app, player, Vec2::new(5.0, 2.0));

    let second = strongest_upward_speed(&mut app, player, 100);

    assert!(
        second >= JumpBlock::STANDARD_LAUNCH_SPEED - 0.7,
        "second fb_jump launch too weak: \
         {second}"
    );

    assert!(app.world().get::<ConsumedFunctionBlock>(jump,).is_none());
}

#[test]
fn disposable_jump_block_launches_high_once_then_disappears() {
    let mut app = app_with_func_map();

    let player = player(&app);
    let jump = entity_at(&app, 9, 0);

    set_player_state(&mut app, player, Vec2::new(9.0, 2.0));

    let first = strongest_upward_speed(&mut app, player, 100);

    assert!(
        first >= JumpBlock::HIGH_LAUNCH_SPEED - 0.7,
        "fb_ds_jump launch too weak: \
         {first}"
    );

    assert!(
        first <= JumpBlock::HIGH_LAUNCH_SPEED + 0.1,
        "fb_ds_jump speed was amplified: \
         {first}"
    );

    assert!(app.world().get::<ConsumedFunctionBlock>(jump,).is_some());

    assert!(app.world().get::<ColliderDisabled>(jump,).is_some());

    assert!(matches!(
        app.world().get::<Visibility>(jump),
        Some(Visibility::Hidden)
    ));

    // 같은 위치에 다시 떨어뜨려도
    // 이미 사라진 고점프 블록은
    // 재발동하지 않아야 합니다.
    set_player_state(&mut app, player, Vec2::new(9.0, 2.0));

    let second = strongest_upward_speed(&mut app, player, 60);

    assert!(
        second < JumpBlock::STANDARD_LAUNCH_SPEED,
        "consumed fb_ds_jump \
         activated again: {second}"
    );
}

#[test]
fn restart_restores_the_disposable_jump_block() {
    let mut app = app_with_func_map();

    let player_entity = player(&app);

    let old_jump = entity_at(&app, 9, 0);

    set_player_state(&mut app, player_entity, Vec2::new(9.0, 2.0));

    let first = strongest_upward_speed(&mut app, player_entity, 100);

    assert!(first >= JumpBlock::HIGH_LAUNCH_SPEED - 0.7);

    assert!(
        app.world()
            .get::<ConsumedFunctionBlock>(old_jump,)
            .is_some()
    );

    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    assert!(!app.world().entities().contains(old_jump));

    let new_jump = entity_at(&app, 9, 0);

    assert_ne!(new_jump, old_jump);

    assert!(app.world().get::<OneShotFunctionBlock>(new_jump,).is_some());

    assert!(
        app.world()
            .get::<ConsumedFunctionBlock>(new_jump,)
            .is_none()
    );

    assert!(app.world().get::<ColliderDisabled>(new_jump,).is_none());

    assert!(app.world().get::<Collider>(new_jump).is_some());

    let new_player = player(&app);

    set_player_state(&mut app, new_player, Vec2::new(9.0, 2.0));

    let second = strongest_upward_speed(&mut app, new_player, 100);

    assert!(
        second >= JumpBlock::HIGH_LAUNCH_SPEED - 0.7,
        "restored fb_ds_jump \
         did not work: {second}"
    );

    assert!(
        app.world()
            .get::<ConsumedFunctionBlock>(new_jump,)
            .is_some()
    );
}
