use super::AbilityUseDirection;
use bevy::prelude::Vec2;

pub const ITEM_STRAIGHT_SPEED: f32 = 10.0;

pub(super) fn straight_ability_direction(direction: AbilityUseDirection) -> Vec2 {
    Vec2::new(direction.horizontal(), 0.0)
}

pub(super) fn straight_ability_velocity(direction: AbilityUseDirection) -> Vec2 {
    straight_ability_direction(direction) * ITEM_STRAIGHT_SPEED
}
