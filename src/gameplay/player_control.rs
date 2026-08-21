use super::{MapSpawnSet, PlaySession, PlaySessionSet, PlayerBall};
use avian2d::prelude::*;
use bevy::{input::InputSystems, prelude::*};

pub const PLAYER_MAX_HORIZONTAL_SPEED: f32 = 5.0;
pub const PLAYER_HORIZONTAL_ACCELERATION: f32 = 30.0;
pub const PLAYER_HORIZONTAL_DECELERATION: f32 = 8.0;
pub const PLAYER_HORIZONTAL_STOP_THRESHOLD: f32 = 0.5;

pub struct PlayerControlPlugin;

impl Plugin for PlayerControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_player_input_intent.after(MapSpawnSet))
            .add_systems(PreUpdate, capture_keyboard_input.after(InputSystems))
            .add_systems(
                PhysicsSchedule,
                apply_horizontal_control
                    .after(PlaySessionSet::AdvanceTime)
                    .before(PhysicsStepSystems::BroadPhase),
            );
    }
}

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
    mut players: Query<(&PlayerInputIntent, &mut LinearVelocity), With<PlayerBall>>,
) {
    if !session.is_playing() {
        return;
    }

    let delta_seconds = time.delta_secs();

    for (intent, mut velocity) in &mut players {
        velocity.0.x = next_horizontal_velocity(velocity.0.x, intent.horizontal(), delta_seconds);
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
    fn held_input_accelerates_at_the_unity_equivalent_rate() {
        assert_close(
            next_horizontal_velocity(0.0, 1.0, PHYSICS_DELTA_SECONDS),
            0.4,
        );
        assert_close(
            next_horizontal_velocity(0.0, -1.0, PHYSICS_DELTA_SECONDS),
            -0.4,
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
            11.6,
        );
    }
}
