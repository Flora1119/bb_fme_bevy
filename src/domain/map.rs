use super::CardinalDirection;
use crate::block::{BlockCategory, BlockId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn offset(self, direction: CardinalDirection) -> Self {
        let (offset_x, offset_y) = direction.unit_offset();

        Self {
            x: self.x + offset_x,
            y: self.y + offset_y,
        }
    }
}

impl From<(i32, i32)> for GridPosition {
    fn from((x, y): (i32, i32)) -> Self {
        Self::new(x, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalPair {
    pub a: Option<GridPosition>,
    pub b: Option<GridPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialSwitchState {
    pub electric: bool,
    pub block_1: bool,
    pub block_2: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedMapSettings {
    pub time_limit_seconds: f32,
    pub show_time_ranking: bool,
    pub required_stars: u32,
    pub size: GridSize,

    pub teleport_1_exit: Option<GridPosition>,
    pub teleport_2_exit: Option<GridPosition>,

    pub portal_1: PortalPair,
    pub portal_2: PortalPair,

    pub initial_switches: InitialSwitchState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBlockOption {
    pub name: String,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBlock {
    pub position: GridPosition,
    pub id: BlockId,
    pub category: BlockCategory,
    pub direction: CardinalDirection,
    pub options: Vec<ValidatedBlockOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedMap {
    pub map_name: String,
    pub author: String,
    pub settings: ValidatedMapSettings,
    pub blocks: Vec<ValidatedBlock>,
}

impl ValidatedMap {
    pub fn block_at(&self, position: GridPosition) -> Option<&ValidatedBlock> {
        self.blocks.iter().find(|block| block.position == position)
    }
}
