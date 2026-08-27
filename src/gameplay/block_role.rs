use crate::domain::CardinalDirection;
use bevy::prelude::{Component, Entity, Vec2};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBall;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectibleStar;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectedStar;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransparentStar;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidBlock;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlySpike;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OneShotFunctionBlock;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumedFunctionBlock;

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct JumpBlock {
    launch_speed: f32,
}

impl JumpBlock {
    pub const STANDARD_LAUNCH_SPEED: f32 = 14.0;
    pub const HIGH_LAUNCH_SPEED: f32 = 16.0;

    pub const fn standard() -> Self {
        Self {
            launch_speed: Self::STANDARD_LAUNCH_SPEED,
        }
    }

    pub const fn high() -> Self {
        Self {
            launch_speed: Self::HIGH_LAUNCH_SPEED,
        }
    }

    pub const fn launch_speed(&self) -> f32 {
        self.launch_speed
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct StraightBlock {
    exit_offset: Vec2,
    speed: f32,
}

impl StraightBlock {
    pub const STANDARD_SPEED: f32 = 12.0;
    pub const HIGH_SPEED: f32 = 15.0;

    pub const fn standard_cardinal(direction: CardinalDirection) -> Self {
        Self {
            exit_offset: cardinal_straight_offset(direction),
            speed: Self::STANDARD_SPEED,
        }
    }

    pub const fn standard_diagonal(direction: CardinalDirection) -> Self {
        Self {
            exit_offset: diagonal_straight_offset(direction),
            speed: Self::STANDARD_SPEED,
        }
    }

    pub const fn high_cardinal(direction: CardinalDirection) -> Self {
        Self {
            exit_offset: cardinal_straight_offset(direction),
            speed: Self::HIGH_SPEED,
        }
    }

    pub const fn high_diagonal(direction: CardinalDirection) -> Self {
        Self {
            exit_offset: diagonal_straight_offset(direction),
            speed: Self::HIGH_SPEED,
        }
    }

    pub const fn exit_offset(self) -> Vec2 {
        self.exit_offset
    }

    pub const fn speed(self) -> f32 {
        self.speed
    }

    pub fn launch_direction(self) -> Vec2 {
        self.exit_offset.normalize_or_zero()
    }
}

const fn cardinal_straight_offset(direction: CardinalDirection) -> Vec2 {
    match direction {
        CardinalDirection::Up => Vec2::new(0.0, 1.0),
        CardinalDirection::Right => Vec2::new(1.0, 0.0),
        CardinalDirection::Down => Vec2::new(0.0, -1.0),
        CardinalDirection::Left => Vec2::new(-1.0, 0.0),
    }
}

const fn diagonal_straight_offset(direction: CardinalDirection) -> Vec2 {
    match direction {
        CardinalDirection::Up => Vec2::new(1.0, 1.0),
        CardinalDirection::Right => Vec2::new(1.0, -1.0),
        CardinalDirection::Down => Vec2::new(-1.0, -1.0),
        CardinalDirection::Left => Vec2::new(-1.0, 1.0),
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct StraightMovement {
    direction: Vec2,
    speed: f32,
    cancel_on_press: bool,
}

impl StraightMovement {
    // 일반 직진 블록:
    // 플레이어 입력으로 직진 해제 가능.
    pub fn new(direction: Vec2, speed: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            speed,
            cancel_on_press: true,
        }
    }

    // Clock 등 강제 발사:
    // 입력으로 직진 해제 불가.
    pub fn press_locked(direction: Vec2, speed: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            speed,
            cancel_on_press: false,
        }
    }

    pub const fn direction(self) -> Vec2 {
        self.direction
    }

    pub const fn speed(self) -> f32 {
        self.speed
    }

    pub const fn can_cancel_on_press(self) -> bool {
        self.cancel_on_press
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct StraightMomentum {
    initial_velocity: Vec2,
    current_velocity: Vec2,
    elapsed_seconds: f32,
    duration_seconds: f32,
}

impl StraightMomentum {
    pub fn new(direction: Vec2, speed: f32, duration_seconds: f32) -> Self {
        let initial_velocity = direction.normalize_or_zero() * speed;

        Self {
            initial_velocity,
            current_velocity: initial_velocity,
            elapsed_seconds: 0.0,
            duration_seconds: duration_seconds.max(f32::EPSILON),
        }
    }

    pub const fn current_velocity(&self) -> Vec2 {
        self.current_velocity
    }

    pub fn advance(&mut self, delta_seconds: f32) -> Vec2 {
        let previous_velocity = self.current_velocity;

        self.elapsed_seconds =
            (self.elapsed_seconds + delta_seconds.max(0.0)).min(self.duration_seconds);

        let remaining_ratio = 1.0 - self.elapsed_seconds / self.duration_seconds;

        self.current_velocity = self.initial_velocity * remaining_ratio.max(0.0);

        // 실제 LinearVelocity에서
        // 이번 틱에 얼마나 변화시킬지 반환
        self.current_velocity - previous_velocity
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed_seconds >= self.duration_seconds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockDirectionMode {
    Dir4,
    Dir8,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockBlock {
    mode: ClockDirectionMode,
}

impl ClockBlock {
    pub const ROTATE_INTERVAL_SECONDS: f32 = 0.3;
    pub const LAUNCH_SPEED: f32 = 15.0;
    pub const LAUNCH_OFFSET_BLOCKS: f32 = 0.5;

    pub const fn dir4() -> Self {
        Self {
            mode: ClockDirectionMode::Dir4,
        }
    }

    pub const fn dir8() -> Self {
        Self {
            mode: ClockDirectionMode::Dir8,
        }
    }

    pub const fn mode(self) -> ClockDirectionMode {
        self.mode
    }

    pub const fn direction_count(self) -> u8 {
        match self.mode {
            ClockDirectionMode::Dir4 => 4,
            ClockDirectionMode::Dir8 => 8,
        }
    }

    pub const fn rotation_step_degrees(self) -> f32 {
        match self.mode {
            ClockDirectionMode::Dir4 => 90.0,
            ClockDirectionMode::Dir8 => 45.0,
        }
    }

    pub fn launch_direction(self, direction_index: u8) -> Vec2 {
        const D: f32 = std::f32::consts::FRAC_1_SQRT_2;

        match self.mode {
            ClockDirectionMode::Dir4 => match direction_index % 4 {
                0 => Vec2::new(0.0, 1.0),
                1 => Vec2::new(1.0, 0.0),
                2 => Vec2::new(0.0, -1.0),
                3 => Vec2::new(-1.0, 0.0),
                _ => unreachable!(),
            },

            ClockDirectionMode::Dir8 => match direction_index % 8 {
                0 => Vec2::new(0.0, 1.0),
                1 => Vec2::new(D, D),
                2 => Vec2::new(1.0, 0.0),
                3 => Vec2::new(D, -D),
                4 => Vec2::new(0.0, -1.0),
                5 => Vec2::new(-D, -D),
                6 => Vec2::new(-1.0, 0.0),
                7 => Vec2::new(-D, D),
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ClockSelection {
    source: Entity,
    direction_index: u8,
    elapsed_seconds: f32,
}

impl ClockSelection {
    pub const fn new(source: Entity) -> Self {
        Self {
            source,
            direction_index: 0,
            elapsed_seconds: 0.0,
        }
    }

    pub const fn source(self) -> Entity {
        self.source
    }

    pub const fn direction_index(self) -> u8 {
        self.direction_index
    }

    pub const fn elapsed_seconds(self) -> f32 {
        self.elapsed_seconds
    }

    pub fn advance(&mut self, delta_seconds: f32, clock: ClockBlock) -> u32 {
        const TIME_EPSILON_SECONDS: f32 = 0.000_001;

        self.elapsed_seconds += delta_seconds.max(0.0);

        let mut rotations = 0;

        while self.elapsed_seconds + TIME_EPSILON_SECONDS >= ClockBlock::ROTATE_INTERVAL_SECONDS {
            self.elapsed_seconds =
                (self.elapsed_seconds - ClockBlock::ROTATE_INTERVAL_SECONDS).max(0.0);

            self.direction_index = (self.direction_index + 1) % clock.direction_count();

            rotations += 1;
        }

        rotations
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockLaunchGuard {
    source: Entity,
}

impl ClockLaunchGuard {
    pub const fn new(source: Entity) -> Self {
        Self { source }
    }

    pub const fn source(self) -> Entity {
        self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entity() -> Entity {
        Entity::from_raw_u32(1).expect("test entity must be valid")
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.000_001,
            "expected {expected}, found {actual}"
        );
    }

    #[test]
    fn clock_selection_starts_pointing_up() {
        let selection = ClockSelection::new(test_entity());

        assert_eq!(selection.direction_index(), 0);
        assert_close(selection.elapsed_seconds(), 0.0);
    }

    #[test]
    fn clock_selection_rotates_after_point_three_seconds() {
        let mut selection = ClockSelection::new(test_entity());
        let clock = ClockBlock::dir4();

        let rotations = selection.advance(0.28, clock);

        assert_eq!(rotations, 0);
        assert_eq!(selection.direction_index(), 0);

        let rotations = selection.advance(0.02, clock);

        assert_eq!(rotations, 1);
        assert_eq!(selection.direction_index(), 1);
    }

    #[test]
    fn dir4_clock_wraps_after_four_rotations() {
        let mut selection = ClockSelection::new(test_entity());
        let clock = ClockBlock::dir4();

        for _ in 0..4 {
            selection.advance(ClockBlock::ROTATE_INTERVAL_SECONDS, clock);
        }

        assert_eq!(selection.direction_index(), 0);
    }

    #[test]
    fn dir8_clock_wraps_after_eight_rotations() {
        let mut selection = ClockSelection::new(test_entity());
        let clock = ClockBlock::dir8();

        for _ in 0..8 {
            selection.advance(ClockBlock::ROTATE_INTERVAL_SECONDS, clock);
        }

        assert_eq!(selection.direction_index(), 0);
    }

    #[test]
    fn clock_selection_preserves_remaining_time() {
        let mut selection = ClockSelection::new(test_entity());
        let clock = ClockBlock::dir4();

        let rotations = selection.advance(0.95, clock);

        assert_eq!(rotations, 3);
        assert_eq!(selection.direction_index(), 3);
        assert_close(selection.elapsed_seconds(), 0.05);
    }

    fn assert_vec2_close(actual: Vec2, expected: Vec2) {
        assert!(
            (actual - expected).length() <= 0.000_001,
            "expected {expected:?}, found {actual:?}"
        );
    }

    #[test]
    fn dir4_clock_launch_direction_follows_the_arrow() {
        let clock = ClockBlock::dir4();

        let expected = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(-1.0, 0.0),
        ];

        for (index, expected_direction) in expected.into_iter().enumerate() {
            assert_vec2_close(clock.launch_direction(index as u8), expected_direction);
        }
    }

    #[test]
    fn dir8_clock_launch_direction_follows_all_eight_arrow_positions() {
        const D: f32 = std::f32::consts::FRAC_1_SQRT_2;
        let clock = ClockBlock::dir8();

        let expected = [
            Vec2::new(0.0, 1.0),
            Vec2::new(D, D),
            Vec2::new(1.0, 0.0),
            Vec2::new(D, -D),
            Vec2::new(0.0, -1.0),
            Vec2::new(-D, -D),
            Vec2::new(-1.0, 0.0),
            Vec2::new(-D, D),
        ];

        for (index, expected_direction) in expected.into_iter().enumerate() {
            assert_vec2_close(clock.launch_direction(index as u8), expected_direction);
        }
    }

    #[test]
    fn clock_launch_straight_movement_is_press_locked() {
        let normal = StraightMovement::new(Vec2::new(1.0, 0.0), 12.0);

        let clock = StraightMovement::press_locked(Vec2::new(1.0, 0.0), 15.0);

        assert!(normal.can_cancel_on_press());
        assert!(!clock.can_cancel_on_press());
    }
}
