use super::PlayerAbility;
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Resource, Debug, Default)]
pub struct AbilityInventory {
    queue: VecDeque<PlayerAbility>,
}

impl AbilityInventory {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn current(&self) -> Option<PlayerAbility> {
        self.queue.front().copied()
    }

    pub fn enqueue(&mut self, ability: PlayerAbility) {
        self.queue.push_back(ability);
    }

    pub fn pop_current(&mut self) -> Option<PlayerAbility> {
        self.queue.pop_front()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}
