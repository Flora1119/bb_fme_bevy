use bevy::prelude::Component;

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

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct JumpBlock {
    launch_speed: f32,
}

impl JumpBlock {
    pub const STANDARD_LAUNCH_SPEED: f32 = 14.0;

    pub const fn standard() -> Self {
        Self {
            launch_speed: Self::STANDARD_LAUNCH_SPEED,
        }
    }

    pub const fn launch_speed(&self) -> f32 {
        self.launch_speed
    }
}
