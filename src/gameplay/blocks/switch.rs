use crate::{
    domain::InitialSwitchState,
    gameplay::{MapSpawnSet, PlayWorld},
};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwitchChannel {
    Electric,
    Block1,
    Block2,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchTrigger {
    channel: SwitchChannel,
}

impl SwitchTrigger {
    pub const fn new(channel: SwitchChannel) -> Self {
        Self { channel }
    }

    pub const fn electric() -> Self {
        Self::new(SwitchChannel::Electric)
    }

    pub const fn block_1() -> Self {
        Self::new(SwitchChannel::Block1)
    }

    pub const fn block_2() -> Self {
        Self::new(SwitchChannel::Block2)
    }

    pub const fn channel(&self) -> SwitchChannel {
        self.channel
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchControlledBlock {
    channel: SwitchChannel,
}

impl SwitchControlledBlock {
    pub const fn new(channel: SwitchChannel) -> Self {
        Self { channel }
    }

    pub const fn electric() -> Self {
        Self::new(SwitchChannel::Electric)
    }

    pub const fn block_1() -> Self {
        Self::new(SwitchChannel::Block1)
    }

    pub const fn block_2() -> Self {
        Self::new(SwitchChannel::Block2)
    }

    pub const fn channel(&self) -> SwitchChannel {
        self.channel
    }
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

pub struct SwitchBlockPlugin;

impl Plugin for SwitchBlockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SwitchState>().add_systems(
            Update,
            initialize_switch_state_from_play_world.after(MapSpawnSet),
        );
    }
}

fn initialize_switch_state_from_play_world(
    play_worlds: Query<&PlayWorld, Added<PlayWorld>>,
    mut switch_state: ResMut<SwitchState>,
) {
    let Some(play_world) = play_worlds.iter().last() else {
        return;
    };

    *switch_state = SwitchState::from(play_world.definition().settings.initial_switches);
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
