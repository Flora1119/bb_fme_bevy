use crate::common::load_validated_map;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        CollectedStar, GameplayPlugin, GridIndex, PHYSICS_HZ, RestartPlayWorld, SpawnValidatedMap,
        StarSwitchState, StarSwitchTrigger, TransparentStar,
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

const STAR_SWITCH_MAP: &str = include_str!("../../assets/maps/phase5c_star_switch.json");

fn app_with_star_switch_map() -> App {
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
        .write_message(SpawnValidatedMap(load_validated_map(STAR_SWITCH_MAP)));

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

fn run_until_star_switch_state(app: &mut App, expected: bool) {
    for _ in 0..120 {
        app.update();

        if app.world().resource::<StarSwitchState>().is_active() == expected {
            return;
        }
    }

    panic!("star switch never reached expected state {expected}");
}

#[test]
fn star_switch_map_starts_inactive() {
    let app = app_with_star_switch_map();

    let switch_1 = entity_at(&app, 4, 2);
    let switch_2 = entity_at(&app, 8, 2);
    let star = entity_at(&app, 6, 2);

    assert!(app.world().get::<StarSwitchTrigger>(switch_1).is_some());

    assert!(app.world().get::<StarSwitchTrigger>(switch_2).is_some());

    assert!(app.world().get::<TransparentStar>(star).is_some());

    assert!(!app.world().resource::<StarSwitchState>().is_active());

    assert!(app.world().get::<ColliderDisabled>(star).is_some());
}

#[test]
fn entering_and_leaving_star_switch_controls_transparent_star() {
    let mut app = app_with_star_switch_map();

    let player = entity_at(&app, 2, 2);
    let star = entity_at(&app, 6, 2);

    set_player_state(&mut app, player, Vec2::new(4.0, 2.0));

    run_until_star_switch_state(&mut app, true);

    // Update schedule의 star 동기화까지 진행.
    app.update();

    assert!(app.world().get::<ColliderDisabled>(star).is_none());

    // 스위치에서 완전히 이탈.
    set_player_state(&mut app, player, Vec2::new(2.0, 8.0));

    run_until_star_switch_state(&mut app, false);

    app.update();

    assert!(app.world().get::<ColliderDisabled>(star).is_some());
}

#[test]
fn collected_transparent_star_never_reactivates_and_restart_restores_it() {
    let mut app = app_with_star_switch_map();

    let star = entity_at(&app, 6, 2);

    // 별 스위치 ON.
    app.world_mut()
        .resource_mut::<StarSwitchState>()
        .set_active(true);

    app.update();

    assert!(app.world().get::<ColliderDisabled>(star).is_none());

    // 실제 별 수집 결과와 동일한 상태를 만듭니다.
    app.world_mut()
        .entity_mut(star)
        .insert((CollectedStar, Visibility::Hidden, ColliderDisabled));

    // OFF -> ON을 다시 반복해도
    // 이미 먹은 별은 부활하면 안 됩니다.
    app.world_mut()
        .resource_mut::<StarSwitchState>()
        .set_active(false);

    app.update();

    app.world_mut()
        .resource_mut::<StarSwitchState>()
        .set_active(true);

    app.update();

    assert!(app.world().get::<CollectedStar>(star).is_some());

    assert!(app.world().get::<ColliderDisabled>(star).is_some());

    assert_eq!(
        app.world().get::<Visibility>(star),
        Some(&Visibility::Hidden),
    );

    // 실제 Restart 경로.
    app.world_mut().write_message(RestartPlayWorld);

    app.update();
    app.update();

    assert!(!app.world().resource::<StarSwitchState>().is_active());

    // PlayWorld가 새로 생성되므로 Entity도 다시 얻습니다.
    let restarted_star = entity_at(&app, 6, 2);

    assert!(app.world().get::<CollectedStar>(restarted_star).is_none());

    assert!(
        app.world()
            .get::<ColliderDisabled>(restarted_star)
            .is_some()
    );
}
