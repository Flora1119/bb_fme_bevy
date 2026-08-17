use bevy::prelude::Component;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBall;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectibleStar;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectedStar;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidBlock;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlySpike;
