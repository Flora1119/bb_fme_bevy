use avian2d::prelude::*;
use bb_fme_bevy::gameplay::{
    CollectedStar, CollectibleStar, GameplayPhysicsPlugin, PHYSICS_HZ, PlaySession,
    PlaySessionPlugin, PlayerBall, StarCollectionPlugin, StarSensorCollider,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

fn app_with_star_collection() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        StarCollectionPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app
}

fn initialize_physics(app: &mut App, player: Entity) {
    // 첫 Update의 Update schedule에서
    // Player와 Star의 physics component를 붙입니다.
    app.update();

    // 이 테스트에서는 수평/수직 이동 자체가 목적이 아니므로
    // 중력 영향을 제거합니다.
    *app.world_mut()
        .get_mut::<GravityScale>(player)
        .expect("player must have GravityScale") = GravityScale(0.0);

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));
}

#[test]
fn player_collects_star_once_and_disables_it() {
    let mut app = app_with_star_collection();

    let player = app
        .world_mut()
        .spawn((PlayerBall, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();

    let star = app
        .world_mut()
        .spawn((
            CollectibleStar,
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Visible,
        ))
        .id();

    initialize_physics(&mut app, player);

    assert!(
        app.world().get::<StarSensorCollider>(star).is_some(),
        "CollectibleStar must receive a sensor collider"
    );

    // 첫 실제 50 Hz physics tick.
    app.update();

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 1);

    assert!(
        app.world().get::<CollectedStar>(star).is_some(),
        "collected star must be marked"
    );

    assert!(matches!(
        app.world().get::<Visibility>(star),
        Some(Visibility::Hidden)
    ));

    assert!(
        app.world().get::<ColliderDisabled>(star).is_some(),
        "collected star collider must be disabled"
    );

    // 같은 위치에서 여러 physics tick이 지나도
    // 같은 별이 다시 카운트되면 안 됩니다.
    for _ in 0..5 {
        app.update();
    }

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 1);
}

#[test]
fn multiple_stars_can_be_collected_in_the_same_tick() {
    let mut app = app_with_star_collection();

    let player = app
        .world_mut()
        .spawn((PlayerBall, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();

    let star_a = app
        .world_mut()
        .spawn((
            CollectibleStar,
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Visible,
        ))
        .id();

    let star_b = app
        .world_mut()
        .spawn((
            CollectibleStar,
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::Visible,
        ))
        .id();

    initialize_physics(&mut app, player);

    app.update();

    assert_eq!(app.world().resource::<PlaySession>().collected_stars(), 2);

    for star in [star_a, star_b] {
        assert!(app.world().get::<CollectedStar>(star).is_some());

        assert!(app.world().get::<ColliderDisabled>(star).is_some());
    }
}
