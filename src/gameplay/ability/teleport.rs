use crate::{domain::GridPosition, gameplay::BLOCK_WORLD_SIZE};
use avian2d::prelude::Position;
use bevy::prelude::*;

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TeleportCheckpoint {
    position: Option<GridPosition>,
}

impl TeleportCheckpoint {
    pub const fn position(&self) -> Option<GridPosition> {
        self.position
    }

    pub fn activate(&mut self, position: GridPosition) {
        self.position = Some(position);
    }

    pub(super) fn clear(&mut self) {
        self.position = None;
    }
}

pub(super) fn teleport_player_to_checkpoint(
    checkpoint: &TeleportCheckpoint,
    position: &mut Position,
    transform: &mut Transform,
) -> bool {
    let Some(grid) = checkpoint.position() else {
        return false;
    };

    let target = Vec2::new(
        grid.x as f32 * BLOCK_WORLD_SIZE,
        grid.y as f32 * BLOCK_WORLD_SIZE,
    );

    position.0 = target;

    transform.translation.x = target.x;
    transform.translation.y = target.y;

    true
}
