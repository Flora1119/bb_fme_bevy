use crate::{
    domain::{CardinalDirection, GridPosition, InitialSwitchState},
    gameplay::{
        BLOCK_WORLD_SIZE, BlockFacing, BlockVisualSet, MapSpawnSet, OriginGridPosition,
        PhysicsInitializationSet, PlayWorld,
    },
};
use avian2d::prelude::{ColliderDisabled, Position};
use bevy::prelude::*;

const SWITCH_CONTROLLED_BLOCK_ENABLED_ALPHA: f32 = 1.0;
const SWITCH_CONTROLLED_BLOCK_DISABLED_ALPHA: f32 = 0.5;

const ELECTRIC_DOOR_MOVE_DURATION_SECONDS: f32 = 0.3;

#[derive(Component, Debug, Clone, Copy)]
struct ElectricDoorMotion {
    start: Vec2,
    target: Vec2,
    elapsed_seconds: f32,
}

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

        SwitchChannel::Electric => Some((
            "sprites/switch/sw_el_on.png",
            "sprites/switch/sw_el_off.png",
        )),
    }
}

fn electric_controlled_visual_paths(
    block: ElectricControlledBlock,
) -> (&'static str, &'static str) {
    match block {
        ElectricControlledBlock::Hazard => ("sprites/switch/el.png", "sprites/switch/el_off.png"),
        ElectricControlledBlock::Door => ("sprites/switch/el_b.png", "sprites/switch/el_b_off.png"),
    }
}

fn electric_door_target_grid_position(
    origin: GridPosition,
    direction: CardinalDirection,
    is_on: bool,
) -> GridPosition {
    if is_on {
        origin
    } else {
        origin.offset(direction.opposite())
    }
}

fn grid_position_to_world(position: GridPosition) -> Vec2 {
    Vec2::new(
        position.x as f32 * BLOCK_WORLD_SIZE,
        position.y as f32 * BLOCK_WORLD_SIZE,
    )
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

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectricControlledBlock {
    Hazard,
    Door,
}

#[derive(Component, Debug, Clone)]
struct ElectricControlledVisualHandles {
    on: Handle<Image>,
    off: Handle<Image>,
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
                prepare_electric_controlled_visual_handles.after(MapSpawnSet),
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
            )
            .add_systems(
                Update,
                sync_electric_controlled_visual_state
                    .after(prepare_electric_controlled_visual_handles)
                    .after(sync_block_switch_trigger_visual_state)
                    .after(BlockVisualSet),
            )
            .add_systems(
                Update,
                begin_electric_door_motion
                    .after(initialize_switch_state_from_play_world)
                    .after(PhysicsInitializationSet),
            )
            .add_systems(
                Update,
                animate_electric_door_motion.after(begin_electric_door_motion),
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

fn prepare_electric_controlled_visual_handles(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    blocks: Query<(Entity, &ElectricControlledBlock), Without<ElectricControlledVisualHandles>>,
) {
    for (entity, block) in &blocks {
        let (on_path, off_path) = electric_controlled_visual_paths(*block);

        commands
            .entity(entity)
            .insert(ElectricControlledVisualHandles {
                on: asset_server.load(on_path),
                off: asset_server.load(off_path),
            });
    }
}

fn sync_block_switch_collision_state(
    mut commands: Commands,
    switch_state: Res<SwitchState>,
    controlled_blocks: Query<(
        Entity,
        &SwitchControlledBlock,
        Option<&ElectricControlledBlock>,
    )>,
) {
    if !switch_state.is_changed() {
        return;
    }

    for (entity, controlled_block, electric_block) in &controlled_blocks {
        let channel = controlled_block.channel();

        let controls_collision = match channel {
            SwitchChannel::Block1 | SwitchChannel::Block2 => true,

            SwitchChannel::Electric => {
                electric_block.is_some_and(|block| *block == ElectricControlledBlock::Hazard)
            }
        };

        if !controls_collision {
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

fn sync_electric_controlled_visual_state(
    switch_state: Res<SwitchState>,
    mut blocks: Query<(&ElectricControlledVisualHandles, &mut Sprite)>,
) {
    if !switch_state.is_changed() {
        return;
    }

    let is_on = switch_state.is_on(SwitchChannel::Electric);

    for (visuals, mut sprite) in &mut blocks {
        let target = match is_on {
            true => &visuals.on,
            false => &visuals.off,
        };

        if sprite.image != *target {
            sprite.image = target.clone();
        }
    }
}

fn begin_electric_door_motion(
    mut commands: Commands,
    switch_state: Res<SwitchState>,
    doors: Query<(
        Entity,
        &ElectricControlledBlock,
        &OriginGridPosition,
        &BlockFacing,
        &Position,
    )>,
) {
    if !switch_state.is_changed() {
        return;
    }

    let is_on = switch_state.is_on(SwitchChannel::Electric);

    for (entity, electric_block, origin, facing, position) in &doors {
        if *electric_block != ElectricControlledBlock::Door {
            continue;
        }

        let target_grid = electric_door_target_grid_position(origin.0, facing.0, is_on);

        let target = grid_position_to_world(target_grid);
        let start = position.0;

        if start.distance_squared(target) <= f32::EPSILON {
            commands.entity(entity).remove::<ElectricDoorMotion>();

            continue;
        }

        commands.entity(entity).insert(ElectricDoorMotion {
            start,
            target,
            elapsed_seconds: 0.0,
        });
    }
}

fn animate_electric_door_motion(
    time: Res<Time>,
    mut commands: Commands,
    mut doors: Query<(
        Entity,
        &mut ElectricDoorMotion,
        &mut Position,
        &mut Transform,
    )>,
) {
    for (entity, mut motion, mut position, mut transform) in &mut doors {
        motion.elapsed_seconds += time.delta_secs();

        let progress =
            (motion.elapsed_seconds / ELECTRIC_DOOR_MOVE_DURATION_SECONDS).clamp(0.0, 1.0);

        let next = motion.start.lerp(motion.target, progress);

        position.0 = next;
        transform.translation.x = next.x;
        transform.translation.y = next.y;

        if progress >= 1.0 {
            position.0 = motion.target;
            transform.translation.x = motion.target.x;
            transform.translation.y = motion.target.y;

            commands.entity(entity).remove::<ElectricDoorMotion>();
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

        assert_eq!(
            switch_trigger_visual_paths(SwitchChannel::Electric),
            Some((
                "sprites/switch/sw_el_on.png",
                "sprites/switch/sw_el_off.png",
            )),
        );
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

    #[test]
    fn electric_off_disables_hazard_but_not_door_collision() {
        let mut app = App::new();

        app.insert_resource(SwitchState::from(InitialSwitchState {
            electric: false,
            block_1: true,
            block_2: true,
        }))
        .add_systems(Update, sync_block_switch_collision_state);

        let hazard = app
            .world_mut()
            .spawn((
                SwitchControlledBlock::electric(),
                ElectricControlledBlock::Hazard,
            ))
            .id();

        let door = app
            .world_mut()
            .spawn((
                SwitchControlledBlock::electric(),
                ElectricControlledBlock::Door,
            ))
            .id();

        app.update();

        assert!(app.world().get::<ColliderDisabled>(hazard).is_some());

        assert!(app.world().get::<ColliderDisabled>(door).is_none());
    }

    #[test]
    fn electric_controlled_visual_paths_match_block_types() {
        assert_eq!(
            electric_controlled_visual_paths(ElectricControlledBlock::Hazard,),
            ("sprites/switch/el.png", "sprites/switch/el_off.png",),
        );

        assert_eq!(
            electric_controlled_visual_paths(ElectricControlledBlock::Door,),
            ("sprites/switch/el_b.png", "sprites/switch/el_b_off.png",),
        );
    }

    #[test]
    fn electric_door_moves_opposite_its_facing_when_off() {
        let origin = GridPosition::new(10, 10);

        assert_eq!(
            electric_door_target_grid_position(origin, CardinalDirection::Up, false,),
            GridPosition::new(10, 9),
        );

        assert_eq!(
            electric_door_target_grid_position(origin, CardinalDirection::Right, false,),
            GridPosition::new(9, 10),
        );

        assert_eq!(
            electric_door_target_grid_position(origin, CardinalDirection::Down, false,),
            GridPosition::new(10, 11),
        );

        assert_eq!(
            electric_door_target_grid_position(origin, CardinalDirection::Left, false,),
            GridPosition::new(11, 10),
        );

        assert_eq!(
            electric_door_target_grid_position(origin, CardinalDirection::Down, true,),
            origin,
        );
    }
}
