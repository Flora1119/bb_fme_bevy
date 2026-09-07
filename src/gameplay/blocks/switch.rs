use crate::domain::InitialSwitchState;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwitchChannel {
    Electric,
    Block1,
    Block2,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchState {
    electric: bool,
    block_1: bool,
    block_2: bool,
}

impl Default for SwitchState {
    fn default() -> Self {
        Self {
            electric: true,
            block_1: true,
            block_2: true,
        }
    }
}

impl From<InitialSwitchState> for SwitchState {
    fn from(initial: InitialSwitchState) -> Self {
        Self {
            electric: initial.electric,
            block_1: initial.block_1,
            block_2: initial.block_2,
        }
    }
}

impl SwitchState {
    pub const fn is_on(&self, channel: SwitchChannel) -> bool {
        match channel {
            SwitchChannel::Electric => self.electric,
            SwitchChannel::Block1 => self.block_1,
            SwitchChannel::Block2 => self.block_2,
        }
    }

    pub fn set(&mut self, channel: SwitchChannel, is_on: bool) {
        match channel {
            SwitchChannel::Electric => self.electric = is_on,
            SwitchChannel::Block1 => self.block_1 = is_on,
            SwitchChannel::Block2 => self.block_2 = is_on,
        }
    }

    pub fn toggle(&mut self, channel: SwitchChannel) -> bool {
        let next = !self.is_on(channel);

        self.set(channel, next);

        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_state_copies_initial_map_state() {
        let state = SwitchState::from(InitialSwitchState {
            electric: false,
            block_1: true,
            block_2: false,
        });

        assert!(!state.is_on(SwitchChannel::Electric));
        assert!(state.is_on(SwitchChannel::Block1));
        assert!(!state.is_on(SwitchChannel::Block2));
    }

    #[test]
    fn switch_state_toggle_only_changes_selected_channel() {
        let mut state = SwitchState::default();

        let next = state.toggle(SwitchChannel::Block1);

        assert!(!next);
        assert!(state.is_on(SwitchChannel::Electric));
        assert!(!state.is_on(SwitchChannel::Block1));
        assert!(state.is_on(SwitchChannel::Block2));

        let next = state.toggle(SwitchChannel::Block1);

        assert!(next);
        assert!(state.is_on(SwitchChannel::Block1));
    }
}
