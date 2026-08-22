use super::map_spawn::BLOCK_WORLD_SIZE;
use bevy::prelude::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpikeRectGeometry {
    size: Vec2,
    offset: Vec2,
}

impl SpikeRectGeometry {
    const fn from_tile_units(width: f32, height: f32, offset_x: f32, offset_y: f32) -> Self {
        Self {
            size: Vec2::new(width * BLOCK_WORLD_SIZE, height * BLOCK_WORLD_SIZE),
            offset: Vec2::new(offset_x * BLOCK_WORLD_SIZE, offset_y * BLOCK_WORLD_SIZE),
        }
    }

    pub const fn size(self) -> Vec2 {
        self.size
    }

    pub const fn offset(self) -> Vec2 {
        self.offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpikeColliderProfile {
    solid: Option<SpikeRectGeometry>,
    damage_sensors: &'static [SpikeRectGeometry],
}

impl SpikeColliderProfile {
    const fn new(
        solid: Option<SpikeRectGeometry>,
        damage_sensors: &'static [SpikeRectGeometry],
    ) -> Self {
        Self {
            solid,
            damage_sensors,
        }
    }

    pub const fn solid(self) -> Option<SpikeRectGeometry> {
        self.solid
    }

    pub const fn damage_sensors(self) -> &'static [SpikeRectGeometry] {
        self.damage_sensors
    }
}

const S_NORMAL_DAMAGE: [SpikeRectGeometry; 1] =
    [SpikeRectGeometry::from_tile_units(0.5, 0.5, 0.0, -0.25)];

const S_HALF_DAMAGE: [SpikeRectGeometry; 1] =
    [SpikeRectGeometry::from_tile_units(0.5, 0.3, 0.0, -0.35)];

const S_B_NORMAL_SOLID: SpikeRectGeometry =
    SpikeRectGeometry::from_tile_units(1.0, 0.5, 0.0, -0.25);

const S_B_NORMAL_DAMAGE: [SpikeRectGeometry; 1] =
    [SpikeRectGeometry::from_tile_units(0.5, 0.2, 0.0, 0.1)];

const S_B_TWO_SOLID: SpikeRectGeometry = SpikeRectGeometry::from_tile_units(0.5, 0.5, 0.25, -0.25);

const S_B_TWO_DAMAGE: [SpikeRectGeometry; 2] = [
    SpikeRectGeometry::from_tile_units(0.25, 0.2, -0.15, -0.25),
    SpikeRectGeometry::from_tile_units(0.2, 0.25, 0.25, 0.15),
];

const S_B_O_HALF_SOLID: SpikeRectGeometry =
    SpikeRectGeometry::from_tile_units(1.0, 0.5, 0.0, -0.25);

const S_B_O_HALF_DAMAGE: [SpikeRectGeometry; 1] =
    [SpikeRectGeometry::from_tile_units(0.8, 0.2, 0.0, 0.1)];

const S_B_O_TWO_SOLID: SpikeRectGeometry =
    SpikeRectGeometry::from_tile_units(0.5, 0.5, 0.25, -0.25);

const S_B_O_TWO_DAMAGE: [SpikeRectGeometry; 2] = [
    SpikeRectGeometry::from_tile_units(0.24, 0.4, -0.12, -0.25),
    SpikeRectGeometry::from_tile_units(0.4, 0.24, 0.25, 0.12),
];

pub fn spike_collider_profile(block_id: &str) -> Option<SpikeColliderProfile> {
    match block_id {
        "s_normal" => Some(SpikeColliderProfile::new(None, &S_NORMAL_DAMAGE)),

        "s_half" => Some(SpikeColliderProfile::new(None, &S_HALF_DAMAGE)),

        "s_b_normal" => Some(SpikeColliderProfile::new(
            Some(S_B_NORMAL_SOLID),
            &S_B_NORMAL_DAMAGE,
        )),

        "s_b_two" => Some(SpikeColliderProfile::new(
            Some(S_B_TWO_SOLID),
            &S_B_TWO_DAMAGE,
        )),

        "s_b_o_half" => Some(SpikeColliderProfile::new(
            Some(S_B_O_HALF_SOLID),
            &S_B_O_HALF_DAMAGE,
        )),

        "s_b_o_two" => Some(SpikeColliderProfile::new(
            Some(S_B_O_TWO_SOLID),
            &S_B_O_TWO_DAMAGE,
        )),

        _ => None,
    }
}

pub fn spike_collider_profile_for(block_id: Option<&str>) -> SpikeColliderProfile {
    block_id
        .and_then(spike_collider_profile)
        .unwrap_or_else(|| SpikeColliderProfile::new(None, &S_NORMAL_DAMAGE))
}

pub fn spike_has_solid_collider(block_id: &str) -> bool {
    matches!(
        block_id,
        "s_b_normal" | "s_b_two" | "s_b_o_half" | "s_b_o_two"
    )
}
