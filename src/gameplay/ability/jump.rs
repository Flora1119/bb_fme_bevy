pub const ITEM_JUMP_POWER: f32 = 12.0;

fn jump_ability_velocity(current: Vec2) -> Vec2 {
    Vec2::new(current.x, ITEM_JUMP_POWER)
}
