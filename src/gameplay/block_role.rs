use crate::domain::CardinalDirection;
use bevy::prelude::{Component, Vec2};

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
}

impl StraightMovement {
    pub fn new(direction: Vec2, speed: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            speed,
        }
    }

    pub const fn direction(self) -> Vec2 {
        self.direction
    }

    pub const fn speed(self) -> f32 {
        self.speed
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct StraightBrake {
    direction: Vec2,
}

impl StraightBrake {
    pub fn new(direction: Vec2) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
        }
    }

    pub const fn direction(self) -> Vec2 {
        self.direction
    }
}
