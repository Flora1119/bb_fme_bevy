use avian2d::prelude::*;
use bb_fme_bevy::gameplay::{
    ActivePlayWorld, GameplayPlugin, GridIndex, PlaySession, PlaySessionState,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, transform::TransformPlugin,
};

#[test]
fn gameplay_plugin_initializes_full_core_physics_schedule_without_ambiguity() {
    let mut app = App::new();

    app.add_plugins((MinimalPlugins, TransformPlugin, InputPlugin, GameplayPlugin));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    // 핵심!
    //
    // PhysicsSchedule을 실제로 초기화하고 실행합니다.
    //
    // Clock + Teleport처럼 같은 Schedule 안에서
    // 시스템 접근 충돌이 생겼는데 실행 순서가 정해져 있지 않다면
    // 여기서 테스트가 panic합니다.
    app.world_mut().run_schedule(PhysicsSchedule);

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing,
    );

    assert!(app.world().resource::<ActivePlayWorld>().root().is_none());

    assert!(app.world().resource::<GridIndex>().is_empty());
}
