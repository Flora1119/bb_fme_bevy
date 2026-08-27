use super::{
    ClockSelection, MapSpawnSet, PLAYER_GRAVITY_SCALE, PlaySession, PlaySessionSet, PlayerBall,
    StraightMomentum, StraightMovement,
};
use avian2d::prelude::*;
use bevy::{input::InputSystems, prelude::*};

pub const PLAYER_MAX_HORIZONTAL_SPEED: f32 = 5.0;
pub const PLAYER_HORIZONTAL_ACCELERATION: f32 = 30.0;
pub const PLAYER_HORIZONTAL_DECELERATION: f32 = 8.0;
pub const PLAYER_HORIZONTAL_STOP_THRESHOLD: f32 = 0.5;

pub const STRAIGHT_MOMENTUM_DURATION: f32 = 0.30;

pub struct PlayerControlPlugin;

impl Plugin for PlayerControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_player_input_intent.after(MapSpawnSet))
            .add_systems(
                PreUpdate,
                (capture_keyboard_input, cancel_straight_movement_on_press)
                    .chain()
                    .in_set(PlayerControlInputSet)
                    .after(InputSystems),
            )
            .add_systems(
                PhysicsSchedule,
                (apply_straight_momentum_decay, apply_horizontal_control)
                    .chain()
                    .after(PlaySessionSet::AdvanceTime)
                    .before(PhysicsStepSystems::BroadPhase),
            );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerControlInputSet;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerInputIntent {
    horizontal: f32,
}

impl PlayerInputIntent {
    pub const fn horizontal(&self) -> f32 {
        self.horizontal
    }

    pub fn set_horizontal(&mut self, horizontal: f32) {
        self.horizontal = horizontal.clamp(-1.0, 1.0);
    }
}

fn attach_player_input_intent(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerBall>, Without<PlayerInputIntent>)>,
) {
    for player in &players {
        commands.entity(player).insert(PlayerInputIntent::default());
    }
}

fn capture_keyboard_input(
    session: Res<PlaySession>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<&mut PlayerInputIntent, With<PlayerBall>>,
) {
    if !session.is_playing() {
        for mut intent in &mut players {
            intent.set_horizontal(0.0);
        }

        return;
    }

    let left_pressed = keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA);

    let right_pressed = keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD);

    let left_just_pressed =
        keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA);

    let right_just_pressed =
        keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);

    for mut intent in &mut players {
        let horizontal = resolve_horizontal_input(
            left_pressed,
            right_pressed,
            left_just_pressed,
            right_just_pressed,
            intent.horizontal(),
        );

        intent.set_horizontal(horizontal);
    }
}

fn apply_horizontal_control(
    session: Res<PlaySession>,
    time: Res<Time<Physics>>,
    mut players: Query<
        (
            &PlayerInputIntent,
            &mut LinearVelocity,
            Option<&StraightMomentum>,
        ),
        (
            With<PlayerBall>,
            Without<StraightMovement>,
            Without<ClockSelection>,
        ),
    >,
) {
    if !session.is_playing() {
        return;
    }

    let delta_seconds = time.delta_secs();

    for (intent, mut velocity, momentum) in &mut players {
        let horizontal = intent.horizontal();

        let momentum_x = momentum.map_or(0.0, |momentum| momentum.current_velocity().x);

        // 실제 속도에서 잔여 직진 관성을
        // 제외한 순수 플레이어 조작 속도.
        let control_velocity_x = velocity.0.x - momentum_x;

        let same_direction =
            horizontal != 0.0 && momentum_x != 0.0 && horizontal.signum() == momentum_x.signum();

        let next_control_velocity_x = if same_direction {
            // 같은 방향 입력일 때는
            // Momentum + Control이 겹쳐
            // 과도하게 밀려나지 않도록 합니다.
            //
            // Momentum 자체가 이미 일반 최고속도보다
            // 빠르면 추가 추진력은 0.
            //
            // Momentum이 5 아래로 떨어지면
            // 부족한 부분만 일반 조작이 이어받습니다.
            let target_control = if momentum_x.abs() >= PLAYER_MAX_HORIZONTAL_SPEED {
                0.0
            } else {
                horizontal * PLAYER_MAX_HORIZONTAL_SPEED - momentum_x
            };

            move_towards(
                control_velocity_x,
                target_control,
                PLAYER_HORIZONTAL_ACCELERATION * delta_seconds,
            )
        } else {
            // 반대 방향이나 Momentum이 없는 경우는
            // 기존 조작을 그대로 사용합니다.
            next_horizontal_velocity(control_velocity_x, horizontal, delta_seconds)
        };

        velocity.0.x += next_control_velocity_x - control_velocity_x;
    }
}

fn resolve_horizontal_input(
    left_pressed: bool,
    right_pressed: bool,
    left_just_pressed: bool,
    right_just_pressed: bool,
    previous: f32,
) -> f32 {
    match (left_pressed, right_pressed) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        (false, false) => 0.0,
        (true, true) => match (left_just_pressed, right_just_pressed) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            (true, true) => 0.0,
            (false, false) => previous.signum(),
        },
    }
}

fn next_horizontal_velocity(current: f32, horizontal: f32, delta_seconds: f32) -> f32 {
    let horizontal = horizontal.clamp(-1.0, 1.0);

    if horizontal != 0.0 {
        let target = horizontal * PLAYER_MAX_HORIZONTAL_SPEED;
        let max_delta = PLAYER_HORIZONTAL_ACCELERATION * delta_seconds;

        accelerate_toward_control_speed(current, target, max_delta)
    } else if current.abs() <= PLAYER_HORIZONTAL_STOP_THRESHOLD {
        0.0
    } else {
        let max_delta = PLAYER_HORIZONTAL_DECELERATION * delta_seconds;

        move_towards(current, 0.0, max_delta)
    }
}

fn accelerate_toward_control_speed(current: f32, target: f32, max_delta: f32) -> f32 {
    let max_delta = max_delta.max(0.0);

    if target > 0.0 {
        if current >= target {
            current
        } else {
            (current + max_delta).min(target)
        }
    } else if current <= target {
        current
    } else {
        (current - max_delta).max(target)
    }
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    let max_delta = max_delta.max(0.0);

    if delta.abs() <= max_delta {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

fn cancel_straight_movement_on_press(
    mut commands: Commands,
    session: Res<PlaySession>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(Entity, &StraightMovement, &mut GravityScale), With<PlayerBall>>,
) {
    if !session.is_playing() {
        return;
    }

    let pressed = keyboard.just_pressed(KeyCode::ArrowLeft)
        || keyboard.just_pressed(KeyCode::KeyA)
        || keyboard.just_pressed(KeyCode::ArrowRight)
        || keyboard.just_pressed(KeyCode::KeyD);

    if !pressed {
        return;
    }

    for (entity, straight, mut gravity_scale) in &mut players {
        if !straight.can_cancel_on_press() {
            continue;
        }

        let momentum = StraightMomentum::new(
            straight.direction(),
            straight.speed(),
            STRAIGHT_MOMENTUM_DURATION,
        );

        // 중력은 즉시 복귀.
        *gravity_scale = GravityScale(PLAYER_GRAVITY_SCALE);

        // 강제 직진은 종료하지만
        // 직진 속도 자체는 관성으로 남깁니다.
        commands
            .entity(entity)
            .remove::<StraightMovement>()
            .insert(momentum);
    }
}

fn apply_straight_momentum_decay(
    mut commands: Commands,
    time: Res<Time<Physics>>,
    mut players: Query<(Entity, &mut StraightMomentum, &mut LinearVelocity), With<PlayerBall>>,
) {
    let delta_seconds = time.delta_secs();

    for (entity, mut momentum, mut velocity) in &mut players {
        let velocity_delta = momentum.advance(delta_seconds);

        // 플레이어 입력이나 중력으로 생긴
        // 속도는 그대로 두고,
        // 직진 관성의 변화량만 반영합니다.
        velocity.0 += velocity_delta;

        if momentum.is_finished() {
            commands.entity(entity).remove::<StraightMomentum>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHYSICS_DELTA_SECONDS: f32 = 1.0 / 50.0;
    const EPSILON: f32 = 0.000_001;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, found {actual}"
        );
    }

    #[test]
    fn keyboard_input_uses_the_most_recent_direction_while_both_are_held() {
        assert_eq!(resolve_horizontal_input(true, true, false, true, -1.0), 1.0);
        assert_eq!(resolve_horizontal_input(true, true, true, false, 1.0), -1.0);
        assert_eq!(
            resolve_horizontal_input(true, true, false, false, -1.0),
            -1.0
        );
    }

    #[test]
    fn held_input_accelerates_at_the_gameplay_tuned_rate() {
        assert_close(
            next_horizontal_velocity(0.0, 1.0, PHYSICS_DELTA_SECONDS),
            0.6,
        );

        assert_close(
            next_horizontal_velocity(0.0, -1.0, PHYSICS_DELTA_SECONDS),
            -0.6,
        );
    }

    #[test]
    fn held_input_reaches_the_control_speed_without_overshooting() {
        assert_close(
            next_horizontal_velocity(4.9, 1.0, PHYSICS_DELTA_SECONDS),
            PLAYER_MAX_HORIZONTAL_SPEED,
        );
        assert_close(
            next_horizontal_velocity(-4.9, -1.0, PHYSICS_DELTA_SECONDS),
            -PLAYER_MAX_HORIZONTAL_SPEED,
        );
    }

    #[test]
    fn released_input_decelerates_and_then_snaps_to_rest() {
        assert_close(
            next_horizontal_velocity(3.0, 0.0, PHYSICS_DELTA_SECONDS),
            2.84,
        );
        assert_close(
            next_horizontal_velocity(-3.0, 0.0, PHYSICS_DELTA_SECONDS),
            -2.84,
        );
        assert_close(
            next_horizontal_velocity(PLAYER_HORIZONTAL_STOP_THRESHOLD, 0.0, PHYSICS_DELTA_SECONDS),
            0.0,
        );
    }

    #[test]
    fn control_speed_does_not_destroy_external_wall_or_ability_velocity() {
        assert_close(
            next_horizontal_velocity(12.0, 1.0, PHYSICS_DELTA_SECONDS),
            12.0,
        );
        assert_close(
            next_horizontal_velocity(-12.0, -1.0, PHYSICS_DELTA_SECONDS),
            -12.0,
        );
        assert_close(
            next_horizontal_velocity(12.0, -1.0, PHYSICS_DELTA_SECONDS),
            11.4,
        );
    }
}
