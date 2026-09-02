use super::{
    BLOCK_WORLD_SIZE, JumpBlock, MapSpawnSet, PendingPlayInteractions, PlayInteraction,
    PlayInteractionCollectSet, PlayInteractionSet, PlaySession, PlayerBall, PlayerControlInputSet,
    PlayerInputIntent, SpawnValidatedMap, StraightBlock, StraightMomentum, StraightMovement,
};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAbility {
    Jump,
    Dash,
    Straight,
    Teleport,
    GravityInvert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityItemEffect {
    Queue(PlayerAbility),
    SetInvisible(bool),
    AdjustGravityScale(GravityScaleAdjustment),
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityItem {
    effect: AbilityItemEffect,
}

impl AbilityItem {
    pub const fn new(effect: AbilityItemEffect) -> Self {
        Self { effect }
    }

    pub const fn effect(self) -> AbilityItemEffect {
        self.effect
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityUseDirection {
    Left,
    Right,
}

impl AbilityUseDirection {
    pub const fn horizontal(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }

    fn from_horizontal(horizontal: f32) -> Option<Self> {
        if horizontal < 0.0 {
            Some(Self::Left)
        } else if horizontal > 0.0 {
            Some(Self::Right)
        } else {
            None
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AbilityUseIntent {
    direction: Option<AbilityUseDirection>,
}

impl AbilityUseIntent {
    pub const fn direction(&self) -> Option<AbilityUseDirection> {
        self.direction
    }

    pub fn request(&mut self, direction: AbilityUseDirection) {
        self.direction = Some(direction);
    }

    pub fn take(&mut self) -> Option<AbilityUseDirection> {
        self.direction.take()
    }

    pub fn clear(&mut self) {
        self.direction = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GravityScaleAdjustment {
    Weaker,
    Stronger,
}
