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

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OneShotFunctionBlock;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumedFunctionBlock;
