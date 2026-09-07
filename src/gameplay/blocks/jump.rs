use bevy::prelude::Component;

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
