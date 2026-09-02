pub const ITEM_STRAIGHT_SPEED: f32 = 10.0;

fn straight_ability_direction(direction: AbilityUseDirection) -> Vec2 {
    Vec2::new(direction.horizontal(), 0.0)
}

fn straight_ability_velocity(direction: AbilityUseDirection) -> Vec2 {
    straight_ability_direction(direction) * ITEM_STRAIGHT_SPEED
}
