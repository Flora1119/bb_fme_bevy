use crate::common::load_validated_map;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        ElectricControlledBlock, GameplayPlugin, GridIndex, PHYSICS_HZ, PlaySession,
        PlaySessionState, RestartPlayWorld, SpawnValidatedMap, SwitchChannel, SwitchState,
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

const ELECTRIC_MAP: &str = include_str!("../../assets/maps/phase5c_electric_switch.json");

fn app_with_electric_map() -> App {
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
        .write_message(SpawnValidatedMap(load_validated_map(ELECTRIC_MAP)));

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

fn set_player_position(app: &mut App, player: Entity, position: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(Vec2::ZERO),
    ));
}

fn run_until_switch_state(app: &mut App, expected: bool) {
    for _ in 0..120 {
        app.update();

        if app
            .world()
            .resource::<SwitchState>()
            .is_on(SwitchChannel::Electric)
            == expected
        {
            return;
        }
    }

    panic!("electric switch never reached {expected}");
}

fn run_until_position(app: &mut App, entity: Entity, expected: Vec2) {
    for _ in 0..30 {
        app.update();

        let actual = app
            .world()
            .get::<Position>(entity)
            .expect("entity must have Position")
            .0;

        if actual.distance(expected) <= 0.001 {
            return;
        }
    }

    let actual = app
        .world()
        .get::<Position>(entity)
        .expect("entity must have Position")
        .0;

    panic!("entity never reached {expected:?}; actual = {actual:?}");
}

#[test]
fn electric_blocks_spawn_in_enabled_state() {
    let app = app_with_electric_map();

    assert!(
        app.world()
            .resource::<SwitchState>()
            .is_on(SwitchChannel::Electric)
    );

    let hazard = entity_at(&app, 6, 1);
    let door = entity_at(&app, 10, 1);

    assert_eq!(
        app.world().get::<ElectricControlledBlock>(hazard),
        Some(&ElectricControlledBlock::Hazard),
    );

    assert_eq!(
        app.world().get::<ElectricControlledBlock>(door),
        Some(&ElectricControlledBlock::Door),
    );

    assert!(app.world().get::<ColliderDisabled>(hazard).is_none());
}

#[test]
fn enabled_electric_hazard_kills_player() {
    let mut app = app_with_electric_map();

    let player = entity_at(&app, 2, 2);

    set_player_position(&mut app, player, Vec2::new(6.0, 1.0));

    for _ in 0..30 {
        app.update();

        if app.world().resource::<PlaySession>().state() == PlaySessionState::Dead {
            return;
        }
    }

    panic!("enabled electric hazard did not kill player");
}

#[test]
fn electric_switch_disables_hazard_and_restart_restores_it() {
    let mut app = app_with_electric_map();

    let player = entity_at(&app, 2, 2);

    // sw_el 착지: ON -> OFF
    set_player_position(&mut app, player, Vec2::new(3.0, 2.0));

    run_until_switch_state(&mut app, false);

    app.update();

    let hazard = entity_at(&app, 6, 1);

    assert!(app.world().get::<ColliderDisabled>(hazard).is_some());

    // OFF된 전기 안으로 들어가도 살아 있어야 합니다.
    set_player_position(&mut app, player, Vec2::new(6.0, 1.0));

    for _ in 0..30 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing,
    );

    // Restart
    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    assert!(
        app.world()
            .resource::<SwitchState>()
            .is_on(SwitchChannel::Electric)
    );

    let hazard = entity_at(&app, 6, 1);

    assert!(app.world().get::<ColliderDisabled>(hazard).is_none());

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing,
    );
}

#[test]
fn electric_door_moves_out_and_back_and_restart_restores_origin() {
    let mut app = app_with_electric_map();

    let player = entity_at(&app, 2, 2);
    let door = entity_at(&app, 10, 1);

    // 초기 Electric ON:
    // 문은 원래 위치 (10, 1)에 있어야 합니다.
    run_until_position(&mut app, door, Vec2::new(10.0, 1.0));

    assert!(app.world().get::<ColliderDisabled>(door).is_none());

    // --------------------------------
    // ON -> OFF
    // dir = Down이므로 반대 방향인 위쪽으로 한 칸 이동.
    // (10, 1) -> (10, 2)
    // --------------------------------
    set_player_position(&mut app, player, Vec2::new(3.0, 2.0));

    run_until_switch_state(&mut app, false);

    // 같은 스위치에 계속 접촉하지 않도록 즉시 떨어뜨립니다.
    set_player_position(&mut app, player, Vec2::new(1.0, 3.0));

    app.update();

    run_until_position(&mut app, door, Vec2::new(10.0, 2.0));

    // 문은 이동해도 Collider가 사라지면 안 됩니다.
    assert!(app.world().get::<ColliderDisabled>(door).is_none());

    // --------------------------------
    // OFF -> ON
    // 원래 위치로 복귀.
    // --------------------------------
    set_player_position(&mut app, player, Vec2::new(3.0, 2.0));

    run_until_switch_state(&mut app, true);

    set_player_position(&mut app, player, Vec2::new(1.0, 3.0));

    app.update();

    run_until_position(&mut app, door, Vec2::new(10.0, 1.0));

    assert!(app.world().get::<ColliderDisabled>(door).is_none());

    // --------------------------------
    // 다시 OFF로 만든 뒤 Restart 검증.
    // --------------------------------
    set_player_position(&mut app, player, Vec2::new(3.0, 2.0));

    run_until_switch_state(&mut app, false);

    set_player_position(&mut app, player, Vec2::new(1.0, 3.0));

    app.update();

    run_until_position(&mut app, door, Vec2::new(10.0, 2.0));

    // Restart
    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    assert!(
        app.world()
            .resource::<SwitchState>()
            .is_on(SwitchChannel::Electric)
    );

    // Restart 뒤에는 Entity가 새로 생성되므로 다시 얻습니다.
    let restarted_door = entity_at(&app, 10, 1);

    run_until_position(&mut app, restarted_door, Vec2::new(10.0, 1.0));

    assert!(
        app.world()
            .get::<ColliderDisabled>(restarted_door)
            .is_none()
    );
}
