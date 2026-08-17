use avian2d::prelude::*;
use bb_fme_bevy::gameplay::{
    GameplayPhysicsPlugin, PHYSICS_HZ, PendingPlayInteractions, PlayInteraction, PlaySession,
    PlaySessionPlugin, PlaySessionState, PlayerBall, PlayerControlPlugin, PlayerInputIntent,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const EPSILON: f32 = 0.000_001;

fn app_with_session() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayerControlPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    // Bevy/Avian의 시간 및 fixed schedule 상태를 먼저 초기화합니다.
    app.update();

    // 이후 app.update()마다 정확히 1/50초가 진행되게 합니다.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {expected}, found {actual}"
    );
}

#[test]
fn death_beats_collection_regardless_of_arrival_order() {
    for collection_first in [true, false] {
        let mut app = app_with_session();

        let star = app.world_mut().spawn_empty().id();
        let spike = app.world_mut().spawn_empty().id();

        {
            let mut pending = app.world_mut().resource_mut::<PendingPlayInteractions>();

            if collection_first {
                pending.push(PlayInteraction::collection(star));

                pending.push(PlayInteraction::death(spike));
            } else {
                pending.push(PlayInteraction::death(spike));

                pending.push(PlayInteraction::collection(star));
            }
        }

        app.update();

        let session = app.world().resource::<PlaySession>();

        assert_eq!(session.state(), PlaySessionState::Dead);

        assert_eq!(session.collected_stars(), 0);
    }
}

#[test]
fn duplicate_collection_from_same_source_counts_once_per_tick() {
    let mut app = app_with_session();

    let star = app.world_mut().spawn_empty().id();

    {
        let mut pending = app.world_mut().resource_mut::<PendingPlayInteractions>();

        pending.push(PlayInteraction::collection(star));

        pending.push(PlayInteraction::collection(star));
    }

    app.update();

    let session = app.world().resource::<PlaySession>();

    assert_eq!(session.state(), PlaySessionState::Playing);

    assert_eq!(session.collected_stars(), 1);
}

#[test]
fn timer_stops_when_session_is_not_playing() {
    let mut app = app_with_session();

    app.update();

    let elapsed_before_death = app.world().resource::<PlaySession>().elapsed_seconds();

    assert!(
        elapsed_before_death > 0.0,
        "timer must advance while playing"
    );

    app.world_mut().resource_mut::<PlaySession>().mark_dead();

    for _ in 0..5 {
        app.update();
    }

    let elapsed_after_death = app.world().resource::<PlaySession>().elapsed_seconds();

    assert_close(elapsed_after_death, elapsed_before_death);
}

#[test]
fn horizontal_control_stops_when_session_is_not_playing() {
    let mut app = app_with_session();

    let player = app
        .world_mut()
        .spawn((
            PlayerBall,
            PlayerInputIntent::default(),
            LinearVelocity(Vec2::ZERO),
        ))
        .id();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);

    app.update();

    let moving_velocity = app
        .world()
        .get::<LinearVelocity>(player)
        .expect("player must have LinearVelocity")
        .0
        .x;

    assert!(
        moving_velocity > 0.0,
        "player must accelerate while Playing"
    );

    app.world_mut().resource_mut::<PlaySession>().mark_dead();

    app.world_mut()
        .get_mut::<LinearVelocity>(player)
        .expect("player must have LinearVelocity")
        .0
        .x = 0.0;

    app.update();

    let stopped_velocity = app
        .world()
        .get::<LinearVelocity>(player)
        .expect("player must have LinearVelocity")
        .0
        .x;

    let intent = app
        .world()
        .get::<PlayerInputIntent>(player)
        .expect("player must have PlayerInputIntent");

    assert_close(stopped_velocity, 0.0);
    assert_close(intent.horizontal(), 0.0);
}
