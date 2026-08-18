use super::{
    ActivePlayWorld, MapSpawnSet, PendingPlayInteractions, PlaySession, PlayWorld,
    SpawnValidatedMap,
};
use bevy::{input::InputSystems, prelude::*};

pub struct PlayRestartPlugin;

impl Plugin for PlayRestartPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RestartPlayWorld>()
            .add_systems(PreUpdate, capture_restart_keyboard.after(InputSystems))
            .add_systems(Update, restart_active_play_world.before(MapSpawnSet));
    }
}

#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RestartPlayWorld;

fn capture_restart_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut restart_requests: MessageWriter<RestartPlayWorld>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        restart_requests.write(RestartPlayWorld);
    }
}

fn restart_active_play_world(
    mut restart_requests: MessageReader<RestartPlayWorld>,
    active_play_world: Res<ActivePlayWorld>,
    play_worlds: Query<&PlayWorld>,
    mut spawn_requests: MessageWriter<SpawnValidatedMap>,
    mut session: ResMut<PlaySession>,
    mut pending_interactions: ResMut<PendingPlayInteractions>,
) {
    // 같은 프레임에 여러 곳에서 Restart 요청이 들어오더라도
    // 실제 재시작은 한 번만 수행합니다.
    if restart_requests.read().count() == 0 {
        return;
    }

    let Some(root) = active_play_world.root() else {
        return;
    };

    let Ok(play_world) = play_worlds.get(root) else {
        return;
    };

    // 현재 실행 중인 PlayWorld가 가진 불변 원본 맵을 사용합니다.
    let map = play_world.definition().clone();

    session.reset();

    *pending_interactions = PendingPlayInteractions::default();

    spawn_requests.write(SpawnValidatedMap(map));
}
