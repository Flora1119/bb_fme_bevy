use crate::common::load_validated_map;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        GameplayPlugin, GridIndex, PHYSICS_HZ, RestartPlayWorld, SpawnValidatedMap, SwitchChannel,
        SwitchControlledBlock, SwitchState, SwitchTrigger,
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

const SWITCH_MAP: &str = include_str!("../../assets/maps/phase5c_switch_blocks.json");

fn app_with_switch_map() -> App {
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
        .write_message(SpawnValidatedMap(load_validated_map(SWITCH_MAP)));

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

fn set_player_state(app: &mut App, player: Entity, position: Vec2, velocity: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(velocity),
    ));
}

fn run_until_switch_state(app: &mut App, channel: SwitchChannel, expected: bool) {
    for _ in 0..120 {
        app.update();

        if app.world().resource::<SwitchState>().is_on(channel) == expected {
            return;
        }
    }

    panic!(
        "switch {:?} never reached expected state {}",
        channel, expected,
    );
}

#[test]
fn block_switches_spawn_with_initial_map_state() {
    let app = app_with_switch_map();

    let switch_b1 = entity_at(&app, 3, 0);
    let b1 = entity_at(&app, 5, 1);

    let switch_b2 = entity_at(&app, 7, 0);
    let b2 = entity_at(&app, 9, 1);

    assert_eq!(
        app.world()
            .get::<SwitchTrigger>(switch_b1)
            .unwrap()
            .channel(),
        SwitchChannel::Block1,
    );

    assert_eq!(
        app.world()
            .get::<SwitchControlledBlock>(b1)
            .unwrap()
            .channel(),
        SwitchChannel::Block1,
    );

    assert_eq!(
        app.world()
            .get::<SwitchTrigger>(switch_b2)
            .unwrap()
            .channel(),
        SwitchChannel::Block2,
    );

    assert_eq!(
        app.world()
            .get::<SwitchControlledBlock>(b2)
            .unwrap()
            .channel(),
        SwitchChannel::Block2,
    );

    let state = app.world().resource::<SwitchState>();

    assert!(state.is_on(SwitchChannel::Block1));
    assert!(!state.is_on(SwitchChannel::Block2));

    assert!(app.world().get::<ColliderDisabled>(b1).is_none());

    assert!(app.world().get::<ColliderDisabled>(b2).is_some());
}

#[test]
fn stepping_block_switches_toggles_only_their_own_channel() {
    let mut app = app_with_switch_map();

    let player = entity_at(&app, 2, 2);

    // B1: ON -> OFF
    set_player_state(&mut app, player, Vec2::new(3.0, 2.0), Vec2::ZERO);

    run_until_switch_state(&mut app, SwitchChannel::Block1, false);

    assert!(
        !app.world()
            .resource::<SwitchState>()
            .is_on(SwitchChannel::Block2)
    );

    // Update 스케줄의 collider 동기화까지 한 번 진행.
    app.update();

    let b1 = entity_at(&app, 5, 1);

    assert!(app.world().get::<ColliderDisabled>(b1).is_some());

    // B2: OFF -> ON
    set_player_state(&mut app, player, Vec2::new(7.0, 2.0), Vec2::ZERO);

    run_until_switch_state(&mut app, SwitchChannel::Block2, true);

    assert!(
        !app.world()
            .resource::<SwitchState>()
            .is_on(SwitchChannel::Block1)
    );

    app.update();

    let b2 = entity_at(&app, 9, 1);

    assert!(app.world().get::<ColliderDisabled>(b2).is_none());
}

#[test]
fn restart_restores_initial_block_switch_state() {
    let mut app = app_with_switch_map();

    let player = entity_at(&app, 2, 2);

    // 먼저 B1을 OFF로 변경.
    set_player_state(&mut app, player, Vec2::new(3.0, 2.0), Vec2::ZERO);

    run_until_switch_state(&mut app, SwitchChannel::Block1, false);

    assert!(
        !app.world()
            .resource::<SwitchState>()
            .is_on(SwitchChannel::Block1)
    );

    // 실제 Restart 경로 사용.
    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    let state = app.world().resource::<SwitchState>();

    assert!(state.is_on(SwitchChannel::Block1));
    assert!(!state.is_on(SwitchChannel::Block2));

    // Restart는 PlayWorld 자체를 원본 ValidatedMap으로 다시 생성합니다.
    // 따라서 새 Entity를 GridIndex에서 다시 얻습니다.
    let b1 = entity_at(&app, 5, 1);
    let b2 = entity_at(&app, 9, 1);

    assert!(app.world().get::<ColliderDisabled>(b1).is_none());

    assert!(app.world().get::<ColliderDisabled>(b2).is_some());
}
