use super::{AbilityInventory, AbilityUseDirection, AbilityUseIntent};
use crate::gameplay::{
    PlaySession, PlayerBall, PlayerGravityState, PlayerInputIntent, SpawnValidatedMap,
    WORLD_GRAVITY,
};
use avian2d::prelude::Gravity;
use bevy::prelude::*;

pub const ABILITY_DOUBLE_TAP_WINDOW_SECONDS: f32 = 0.3;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct PlayerAbilityInputSet;

pub(super) fn attach_ability_use_intent(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerBall>, Without<AbilityUseIntent>)>,
) {
    for player in &players {
        commands.entity(player).insert(AbilityUseIntent::default());
    }
}

pub(super) fn reset_ability_state_for_spawn(
    mut spawn_requests: MessageReader<SpawnValidatedMap>,
    mut inventory: ResMut<AbilityInventory>,
    mut double_tap: ResMut<AbilityDoubleTapState>,
    mut gravity_state: ResMut<PlayerGravityState>,
    mut world_gravity: ResMut<Gravity>,
) {
    if spawn_requests.read().next().is_some() {
        inventory.clear();
        double_tap.clear();

        gravity_state.reset();
        world_gravity.0 = WORLD_GRAVITY;
    }
}

#[derive(Resource, Debug, Default)]
pub(super) struct AbilityDoubleTapState {
    last_direction: Option<AbilityUseDirection>,
    elapsed_seconds: f32,
}

impl AbilityDoubleTapState {
    fn advance(&mut self, delta_seconds: f32) {
        if self.last_direction.is_none() {
            return;
        }

        self.elapsed_seconds += delta_seconds.max(0.0);

        if self.elapsed_seconds > ABILITY_DOUBLE_TAP_WINDOW_SECONDS {
            self.clear();
        }
    }

    fn register_press(&mut self, direction: AbilityUseDirection) -> bool {
        let is_double_tap = self.last_direction == Some(direction)
            && self.elapsed_seconds <= ABILITY_DOUBLE_TAP_WINDOW_SECONDS;

        if is_double_tap {
            // 성공한 더블 탭은 소비합니다.
            //
            // 그래서 Left -> Left -> Left 입력이
            // 두 번째와 세 번째 입력까지 연속 발동시키지 않습니다.
            self.clear();
            true
        } else {
            self.last_direction = Some(direction);
            self.elapsed_seconds = 0.0;
            false
        }
    }

    fn clear(&mut self) {
        self.last_direction = None;
        self.elapsed_seconds = 0.0;
    }
}

pub(super) fn capture_ability_use_input(
    session: Res<PlaySession>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    inventory: Res<AbilityInventory>,
    mut double_tap: ResMut<AbilityDoubleTapState>,
    mut players: Query<(&PlayerInputIntent, &mut AbilityUseIntent), With<PlayerBall>>,
) {
    // Intent는 한 프레임짜리 요청입니다.
    for (_, mut ability_intent) in &mut players {
        ability_intent.clear();
    }

    // 플레이 중이 아니거나 보유 능력이 없다면
    // 이전 탭 기록도 남기지 않습니다.
    //
    // 예:
    // 아이템 없는 상태에서 Left
    // -> 바로 i_jump 획득
    // -> Left
    //
    // 이것을 더블 탭으로 오인하면 안 됩니다.
    if !session.is_playing() || inventory.is_empty() {
        double_tap.clear();
        return;
    }

    double_tap.advance(time.delta_secs());

    let left_just_pressed =
        keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA);

    let right_just_pressed =
        keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);

    let double_tap_direction = match (left_just_pressed, right_just_pressed) {
        (true, false) => {
            let direction = AbilityUseDirection::Left;

            double_tap.register_press(direction).then_some(direction)
        }

        (false, true) => {
            let direction = AbilityUseDirection::Right;

            double_tap.register_press(direction).then_some(direction)
        }

        (true, true) => {
            // 좌우가 동시에 눌렸다면 방향이 모호하므로
            // 더블 탭 기록을 취소합니다.
            double_tap.clear();
            None
        }

        (false, false) => None,
    };

    let space_just_pressed = keyboard.just_pressed(KeyCode::Space);

    for (movement_intent, mut ability_intent) in &mut players {
        // 원작의 보조 입력:
        //
        // 방향키를 누른 상태에서 Space를 누르면
        // 더블 탭 없이도 현재 능력을 사용합니다.
        let space_direction = if space_just_pressed {
            AbilityUseDirection::from_horizontal(movement_intent.horizontal())
        } else {
            None
        };

        // 같은 프레임에 둘 다 발생하면
        // 더블 탭 입력을 우선합니다.
        if let Some(direction) = double_tap_direction.or(space_direction) {
            ability_intent.request(direction);
        }
    }
}
