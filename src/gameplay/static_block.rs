use super::map_spawn::BLOCK_WORLD_SIZE;
use bevy::prelude::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidColliderGeometry {
    size: Vec2,
    offset: Vec2,
}

impl SolidColliderGeometry {
    pub const FULL_TILE: Self = Self {
        size: Vec2::new(BLOCK_WORLD_SIZE, BLOCK_WORLD_SIZE),
        offset: Vec2::new(0.0, 0.0),
    };

    pub const BOTTOM_HALF: Self = Self {
        size: Vec2::new(BLOCK_WORLD_SIZE, BLOCK_WORLD_SIZE * 0.5),
        offset: Vec2::new(0.0, -BLOCK_WORLD_SIZE * 0.25),
    };

    pub const BOTTOM_RIGHT_QUARTER: Self = Self {
        size: Vec2::new(BLOCK_WORLD_SIZE * 0.5, BLOCK_WORLD_SIZE * 0.5),
        offset: Vec2::new(BLOCK_WORLD_SIZE * 0.25, -BLOCK_WORLD_SIZE * 0.25),
    };

    pub const fn size(self) -> Vec2 {
        self.size
    }

    pub const fn offset(self) -> Vec2 {
        self.offset
    }
}

pub fn static_block_collider_geometry(block_id: &str) -> Option<SolidColliderGeometry> {
    match block_id {
        "b_normal" => Some(SolidColliderGeometry::FULL_TILE),
        "b_o" => Some(SolidColliderGeometry::FULL_TILE),
        "b_o_half" => Some(SolidColliderGeometry::BOTTOM_HALF),
        "b_o_quarter" => Some(SolidColliderGeometry::BOTTOM_RIGHT_QUARTER),
        _ => None,
    }
}

pub fn solid_collider_geometry_for(block_id: Option<&str>) -> SolidColliderGeometry {
    block_id
        .and_then(static_block_collider_geometry)
        .unwrap_or(SolidColliderGeometry::FULL_TILE)
}
