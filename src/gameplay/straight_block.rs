use crate::domain::CardinalDirection;
use bevy::prelude::{Component, Vec2};

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
        // 이번 틱에 얼마나 변화시킬지 반환.
        self.current_velocity - previous_velocity
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed_seconds >= self.duration_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_locked_straight_movement_cannot_be_cancelled_by_press() {
        let normal = StraightMovement::new(Vec2::X, 12.0);
        let press_locked = StraightMovement::press_locked(Vec2::X, 15.0);

        assert!(normal.can_cancel_on_press());
        assert!(!press_locked.can_cancel_on_press());
    }
}
