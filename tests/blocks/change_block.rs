use crate::common::load_validated_map;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        ChangeBlock, ChangeBlockInnerCollider, ChangeBlockPhase, ChangeBlockTriggerSensor,
        GameplayPlugin, GridIndex, PHYSICS_HZ, PlaySession, PlaySessionState, RestartPlayWorld,
        SolidBlock, SpawnValidatedMap,
    },
};
use bevy::{
    asset::{AssetApp, AssetPlugin, Assets},
    gizmos::GizmoAsset,
    image::{CompressedImageFormats, ImageLoader},
    input::InputPlugin,
    prelude::*,
    time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const CHANGE_MAP: &str = include_str!("../../assets/maps/phase5c_change_block.json");

fn app_with_change_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..default()
        },
        ImagePlugin::default_nearest(),
        TransformPlugin,
        InputPlugin,
        GameplayPlugin,
    ));

    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));

    app.init_resource::<Assets<GizmoAsset>>();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map(CHANGE_MAP)));

    app.finish();
    app.cleanup();

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
        .unwrap_or_else(|| panic!("entity must exist at ({x}, {y})"))
}

fn set_player_state(app: &mut App, player: Entity, position: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity::ZERO,
    ));
}

fn inner_collider_for(app: &mut App, source: Entity) -> Entity {
    let mut query = {
        let world = app.world_mut();

        world.query::<(Entity, &ChangeBlockInnerCollider)>()
    };

    query
        .iter(app.world())
        .find_map(|(entity, collider)| (collider.source() == source).then_some(entity))
        .expect("change block inner collider must exist")
}

fn trigger_sensor_for(app: &mut App, source: Entity) -> Entity {
    let mut query = {
        let world = app.world_mut();

        world.query::<(Entity, &ChangeBlockTriggerSensor)>()
    };

    query
        .iter(app.world())
        .find_map(|(entity, sensor)| (sensor.source() == source).then_some(entity))
        .expect("change block trigger must exist")
}

fn run_until_phase(app: &mut App, block: Entity, expected: ChangeBlockPhase) {
    for _ in 0..150 {
        app.update();

        if app
            .world()
            .get::<ChangeBlock>(block)
            .is_some_and(|block| block.phase() == expected)
        {
            return;
        }
    }

    panic!("change block never reached phase {expected:?}");
}

#[test]
fn change_block_starts_dormant_and_passable() {
    let mut app = app_with_change_map();

    let block = entity_at(&app, 5, 2);

    let inner = inner_collider_for(&mut app, block);

    let trigger = trigger_sensor_for(&mut app, block);

    assert_eq!(
        app.world().get::<ChangeBlock>(block).unwrap().phase(),
        ChangeBlockPhase::Dormant
    );

    assert!(app.world().get::<SolidBlock>(block).is_none());

    assert!(app.world().get::<ColliderDisabled>(inner).is_some());

    // 작동용 0.2 센서는 켜져 있어야 합니다.
    assert!(app.world().get::<ColliderDisabled>(trigger).is_none());

    assert!(app.world().get::<Sensor>(trigger).is_some());
}

#[test]
fn change_block_becomes_deadly_during_second_half() {
    let mut app = app_with_change_map();

    let player = entity_at(&app, 2, 2);
    let block = entity_at(&app, 5, 2);

    // 중앙 Trigger에 접촉.
    set_player_state(&mut app, player, Vec2::new(5.0, 2.0));

    run_until_phase(&mut app, block, ChangeBlockPhase::Changing);

    // 바로 빠져서 안전 지역으로 이동.
    set_player_state(&mut app, player, Vec2::new(2.0, 8.0));

    // 아직 1초 이전에는 죽지 않아야 함.
    for _ in 0..30 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing
    );

    assert!(!app.world().get::<ChangeBlock>(block).unwrap().is_damaging());

    // 후반 Damage 구간까지 진행.
    for _ in 0..30 {
        app.update();
    }

    assert!(app.world().get::<ChangeBlock>(block).unwrap().is_damaging());

    // 위험해진 변화 블록에 재진입.
    set_player_state(&mut app, player, Vec2::new(5.0, 2.0));

    for _ in 0..10 {
        app.update();

        if app.world().resource::<PlaySession>().state() == PlaySessionState::Dead {
            break;
        }
    }

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Dead
    );
}

#[test]
fn change_block_becomes_solid_and_restart_restores_dormant_state() {
    let mut app = app_with_change_map();

    let player = entity_at(&app, 2, 2);
    let block = entity_at(&app, 5, 2);

    set_player_state(&mut app, player, Vec2::new(5.0, 2.0));

    run_until_phase(&mut app, block, ChangeBlockPhase::Changing);

    // Damage 구간에서 죽지 않도록 멀리 이동.
    set_player_state(&mut app, player, Vec2::new(2.0, 8.0));

    run_until_phase(&mut app, block, ChangeBlockPhase::Solid);

    // Update의 물리 상태 동기화까지 한 번.
    app.update();

    let inner = inner_collider_for(&mut app, block);

    let trigger = trigger_sensor_for(&mut app, block);

    assert!(app.world().get::<SolidBlock>(block).is_some());

    assert!(app.world().get::<ColliderDisabled>(inner).is_none());

    // 최종 상태는 Damage Sensor가 아니라 실제 Solid.
    assert!(app.world().get::<Sensor>(inner).is_none());

    // 최초 작동용 작은 센서는 이제 꺼짐.
    assert!(app.world().get::<ColliderDisabled>(trigger).is_some());

    // 실제 Restart 경로.
    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    // PlayWorld가 재생성되므로 Entity도 새로 얻습니다.
    let restarted = entity_at(&app, 5, 2);

    let restarted_inner = inner_collider_for(&mut app, restarted);

    let restarted_trigger = trigger_sensor_for(&mut app, restarted);

    assert_eq!(
        app.world().get::<ChangeBlock>(restarted).unwrap().phase(),
        ChangeBlockPhase::Dormant
    );

    assert!(app.world().get::<SolidBlock>(restarted).is_none());

    assert!(
        app.world()
            .get::<ColliderDisabled>(restarted_inner)
            .is_some()
    );

    assert!(
        app.world()
            .get::<ColliderDisabled>(restarted_trigger)
            .is_none()
    );
}
