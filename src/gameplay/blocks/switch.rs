use crate::{
    domain::InitialSwitchState,
    gameplay::{BlockVisualSet, MapSpawnSet, PhysicsInitializationSet, PlayWorld},
};
use avian2d::prelude::ColliderDisabled;
use bevy::prelude::*;

const SWITCH_CONTROLLED_BLOCK_ENABLED_ALPHA: f32 = 1.0;
const SWITCH_CONTROLLED_BLOCK_DISABLED_ALPHA: f32 = 0.5;

fn switch_trigger_visual_paths(channel: SwitchChannel) -> Option<(&'static str, &'static str)> {
    match channel {
        SwitchChannel::Block1 => Some((
            "sprites/switch/sw_b1_on.png",
            "sprites/switch/sw_b1_off.png",
        )),
        SwitchChannel::Block2 => Some((
            "sprites/switch/sw_b2_on.png",
            "sprites/switch/sw_b2_off.png",
        )),

        // 5-C-2에서 추가
        SwitchChannel::Electric => None,
    }
}

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

#[derive(Component, Debug, Clone)]
struct SwitchTriggerVisualHandles {
    on: Handle<Image>,
    off: Handle<Image>,
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
        app.init_resource::<SwitchState>()
            .add_systems(
                Update,
                initialize_switch_state_from_play_world.after(MapSpawnSet),
            )
            .add_systems(
                Update,
                prepare_switch_trigger_visual_handles.after(MapSpawnSet),
            )
            .add_systems(
                Update,
                sync_block_switch_collision_state
                    .after(initialize_switch_state_from_play_world)
                    .after(PhysicsInitializationSet),
            )
            .add_systems(
                Update,
                sync_block_switch_visual_state
                    .after(initialize_switch_state_from_play_world)
                    .after(BlockVisualSet),
            )
            .add_systems(
                Update,
                sync_block_switch_trigger_visual_state
                    .after(prepare_switch_trigger_visual_handles)
                    .after(sync_block_switch_visual_state)
                    .after(BlockVisualSet),
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

fn prepare_switch_trigger_visual_handles(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    triggers: Query<(Entity, &SwitchTrigger), Without<SwitchTriggerVisualHandles>>,
) {
    for (entity, trigger) in &triggers {
        let Some((on_path, off_path)) = switch_trigger_visual_paths(trigger.channel()) else {
            continue;
        };

        commands.entity(entity).insert(SwitchTriggerVisualHandles {
            on: asset_server.load(on_path),
            off: asset_server.load(off_path),
        });
    }
}

fn sync_block_switch_collision_state(
    mut commands: Commands,
    switch_state: Res<SwitchState>,
    controlled_blocks: Query<(Entity, &SwitchControlledBlock)>,
) {
    if !switch_state.is_changed() {
        return;
    }

    for (entity, controlled_block) in &controlled_blocks {
        let channel = controlled_block.channel();

        if !matches!(channel, SwitchChannel::Block1 | SwitchChannel::Block2) {
            continue;
        }

        if switch_state.is_on(channel) {
            commands.entity(entity).remove::<ColliderDisabled>();
        } else {
            commands.entity(entity).insert(ColliderDisabled);
        }
    }
}

fn sync_block_switch_visual_state(
    switch_state: Res<SwitchState>,
    mut controlled_blocks: Query<(&SwitchControlledBlock, &mut Sprite)>,
) {
    if !switch_state.is_changed() {
        return;
    }

    for (controlled_block, mut sprite) in &mut controlled_blocks {
        let channel = controlled_block.channel();

        if !matches!(channel, SwitchChannel::Block1 | SwitchChannel::Block2) {
            continue;
        }

        let alpha = if switch_state.is_on(channel) {
            SWITCH_CONTROLLED_BLOCK_ENABLED_ALPHA
        } else {
            SWITCH_CONTROLLED_BLOCK_DISABLED_ALPHA
        };

        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
    }
}

fn sync_block_switch_trigger_visual_state(
    switch_state: Res<SwitchState>,
    mut triggers: Query<(&SwitchTrigger, &SwitchTriggerVisualHandles, &mut Sprite)>,
) {
    if !switch_state.is_changed() {
        return;
    }

    for (trigger, visuals, mut sprite) in &mut triggers {
        let target = match switch_state.is_on(trigger.channel()) {
            true => &visuals.on,
            false => &visuals.off,
        };

        // 이미 올바른 이미지라면 아무것도 하지 않음.
        if sprite.image != *target {
            sprite.image = target.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_trigger_visual_paths_match_block_switch_channels() {
        assert_eq!(
            switch_trigger_visual_paths(SwitchChannel::Block1),
            Some((
                "sprites/switch/sw_b1_on.png",
                "sprites/switch/sw_b1_off.png",
            )),
        );

        assert_eq!(
            switch_trigger_visual_paths(SwitchChannel::Block2),
            Some((
                "sprites/switch/sw_b2_on.png",
                "sprites/switch/sw_b2_off.png",
            )),
        );

        assert_eq!(switch_trigger_visual_paths(SwitchChannel::Electric), None,);
    }

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

    #[test]
    fn controlled_block_state_disables_collision_and_dims_visual_when_off() {
        let mut app = App::new();

        app.insert_resource(SwitchState::from(InitialSwitchState {
            electric: true,
            block_1: false,
            block_2: true,
        }))
        .add_systems(
            Update,
            (
                sync_block_switch_collision_state,
                sync_block_switch_visual_state,
            ),
        );

        let block_1 = app
            .world_mut()
            .spawn((SwitchControlledBlock::block_1(), Sprite::default()))
            .id();

        let block_2 = app
            .world_mut()
            .spawn((SwitchControlledBlock::block_2(), Sprite::default()))
            .id();

        app.update();

        assert!(app.world().get::<ColliderDisabled>(block_1).is_some());

        assert!(app.world().get::<ColliderDisabled>(block_2).is_none());

        assert_eq!(
            app.world().get::<Sprite>(block_1).unwrap().color,
            Color::srgba(1.0, 1.0, 1.0, 0.5),
        );

        assert_eq!(
            app.world().get::<Sprite>(block_2).unwrap().color,
            Color::srgba(1.0, 1.0, 1.0, 1.0),
        );
    }
}
