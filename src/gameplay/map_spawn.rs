use super::{
    ChangeBlock, ClockBlock, CollectibleStar, DeadlySpike, ElectricControlledBlock,
    FallingRockSpawner, JumpBlock, OneShotFunctionBlock, PlayerBall, PushPullAction, PushPullBlock,
    PushPullReach, SolidBlock, StarSwitchTrigger, StraightBlock, StraightTransportBlock,
    SwitchControlledBlock, SwitchTrigger, TeleportEntrance, TeleportExit, TransparentStar,
    ability_item_for_id, spike_has_solid_collider,
};
use crate::{
    block::{BlockCategory, BlockId},
    domain::{CardinalDirection, GridPosition, ValidatedBlockOption, ValidatedMap},
};
use bevy::prelude::*;
use std::collections::HashMap;

pub const BLOCK_WORLD_SIZE: f32 = 1.0;

pub struct MapSpawnPlugin;

impl Plugin for MapSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivePlayWorld>()
            .init_resource::<GridIndex>()
            .add_message::<SpawnValidatedMap>()
            .add_systems(Update, spawn_requested_map.in_set(MapSpawnSet));
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapSpawnSet;

#[derive(Message, Debug, Clone)]
pub struct SpawnValidatedMap(pub ValidatedMap);

#[derive(Resource, Debug, Default)]
pub struct ActivePlayWorld {
    root: Option<Entity>,
}

impl ActivePlayWorld {
    pub const fn root(&self) -> Option<Entity> {
        self.root
    }
}

#[derive(Resource, Debug, Default)]
pub struct GridIndex(HashMap<GridPosition, Entity>);

impl GridIndex {
    pub fn entity_at(&self, position: GridPosition) -> Option<Entity> {
        self.0.get(&position).copied()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// 블록의 점유 칸과 현재 논리 좌표를 함께 변경합니다.
    ///
    /// 출발 칸의 소유자가 다르거나 목적지가 점유되어 있으면
    /// 아무것도 변경하지 않고 false를 반환합니다.
    ///
    /// 맵 경계와 이동 가능 방향은 호출하는 이동 규칙에서 검사합니다.
    /// Transform과 물리 위치는 별도의 이동 시스템에서 갱신합니다.
    pub fn try_relocate(
        &mut self,
        entity: Entity,
        current: &mut CurrentGridPosition,
        destination: GridPosition,
    ) -> bool {
        let source = current.0;

        if self.entity_at(source) != Some(entity) {
            return false;
        }

        if source == destination {
            return true;
        }

        if self.entity_at(destination).is_some() {
            return false;
        }

        self.0.remove(&source);
        self.0.insert(destination, entity);
        current.0 = destination;

        true
    }
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct PlayWorld {
    definition: ValidatedMap,
}

impl PlayWorld {
    pub fn definition(&self) -> &ValidatedMap {
        &self.definition
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeBlock;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct BlockIdentity {
    pub id: BlockId,
    pub category: BlockCategory,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockFacing(pub CardinalDirection);

#[derive(Component, Debug, Clone, PartialEq)]
pub struct BlockOptions(pub Vec<ValidatedBlockOption>);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentGridPosition(pub GridPosition);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginGridPosition(pub GridPosition);

fn spawn_requested_map(
    mut commands: Commands,
    mut requests: MessageReader<SpawnValidatedMap>,
    mut active_play_world: ResMut<ActivePlayWorld>,
    mut grid_index: ResMut<GridIndex>,
) {
    let Some(SpawnValidatedMap(map)) = requests.read().last() else {
        return;
    };

    if let Some(previous_root) = active_play_world.root.take() {
        commands.entity(previous_root).despawn();
    }

    let root = commands
        .spawn((
            Name::new(format!("PlayWorld: {}", map.map_name)),
            PlayWorld {
                definition: map.clone(),
            },
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();

    let mut next_grid_index = HashMap::with_capacity(map.blocks.len());

    for block in &map.blocks {
        let position = block.position;
        let angle = (block.direction.unity_angle_degrees() as f32).to_radians();

        let mut entity_commands = commands.spawn((
            Name::new(format!(
                "Block: {} @ ({}, {})",
                block.id, position.x, position.y
            )),
            RuntimeBlock,
            BlockIdentity {
                id: block.id.clone(),
                category: block.category,
            },
            BlockFacing(block.direction),
            BlockOptions(block.options.clone()),
            CurrentGridPosition(position),
            OriginGridPosition(position),
            Transform::from_xyz(
                position.x as f32 * BLOCK_WORLD_SIZE,
                position.y as f32 * BLOCK_WORLD_SIZE,
                0.0,
            )
            .with_rotation(Quat::from_rotation_z(angle)),
            ChildOf(root),
        ));

        match block.category {
            BlockCategory::Block => {
                entity_commands.insert(SolidBlock);
            }
            BlockCategory::Spike => {
                entity_commands.insert(DeadlySpike);

                if spike_has_solid_collider(block.id.as_str()) {
                    entity_commands.insert(SolidBlock);
                }
            }
            _ => {}
        }

        if block.category == BlockCategory::Item {
            if let Some(ability_item) = ability_item_for_id(block.id.as_str()) {
                entity_commands.insert(ability_item);
            }
        }

        match block.id.as_str() {
            "ball" => {
                entity_commands.insert(PlayerBall);
            }

            "star" => {
                entity_commands.insert(CollectibleStar);
            }

            "star_empty" => {
                entity_commands.insert((CollectibleStar, TransparentStar));
            }

            "fb_fr" => {
                entity_commands.insert((
                    FallingRockSpawner::from_options(&block.options),
                    Visibility::Hidden,
                ));
            }

            "fb_jump" => {
                entity_commands.insert((SolidBlock, JumpBlock::standard()));
            }

            "fb_ds_jump" => {
                entity_commands.insert((SolidBlock, JumpBlock::high(), OneShotFunctionBlock));
            }
            "fb_st_hv" => {
                entity_commands.insert((
                    SolidBlock,
                    StraightBlock::standard_cardinal(block.direction),
                ));
            }

            "fb_st_dg" => {
                entity_commands.insert((
                    SolidBlock,
                    StraightBlock::standard_diagonal(block.direction),
                ));
            }

            "fb_ds_st_hv" => {
                entity_commands.insert((
                    SolidBlock,
                    StraightBlock::high_cardinal(block.direction),
                    OneShotFunctionBlock,
                ));
            }

            "fb_ds_st_dg" => {
                entity_commands.insert((
                    SolidBlock,
                    StraightBlock::high_diagonal(block.direction),
                    OneShotFunctionBlock,
                ));
            }
            "fb_clock_d4" => {
                entity_commands.insert(ClockBlock::dir4());
            }

            "fb_clock_d8" => {
                entity_commands.insert(ClockBlock::dir8());
            }

            "fb_tp1_in" => {
                entity_commands.insert(TeleportEntrance::one());
            }

            "fb_tp1_out" => {
                entity_commands.insert(TeleportExit::one());
            }

            "fb_tp2_in" => {
                entity_commands.insert(TeleportEntrance::two());
            }

            "fb_tp2_out" => {
                entity_commands.insert(TeleportExit::two());
            }

            "sw_el_on" | "sw_el_off" => {
                entity_commands.insert((SolidBlock, SwitchTrigger::electric()));
            }

            "el" | "el_off" => {
                entity_commands.insert((
                    SwitchControlledBlock::electric(),
                    ElectricControlledBlock::Hazard,
                ));
            }

            "el_b" | "el_b_off" => {
                entity_commands.insert((
                    SolidBlock,
                    SwitchControlledBlock::electric(),
                    ElectricControlledBlock::Door,
                ));
            }

            "sw_b1_on" | "sw_b1_off" => {
                entity_commands.insert((SolidBlock, SwitchTrigger::block_1()));
            }

            "b1" => {
                entity_commands.insert((SolidBlock, SwitchControlledBlock::block_1()));
            }

            "sw_b2_on" | "sw_b2_off" => {
                entity_commands.insert((SolidBlock, SwitchTrigger::block_2()));
            }

            "b2" => {
                entity_commands.insert((SolidBlock, SwitchControlledBlock::block_2()));
            }

            "wb_star_sw" => {
                entity_commands.insert(StarSwitchTrigger);
            }

            "wb_change" => {
                entity_commands.insert(ChangeBlock::default());
            }

            "tp_st_o" => {
                entity_commands.insert((SolidBlock, StraightTransportBlock::one_step()));
            }

            "tp_st_a" => {
                entity_commands.insert((SolidBlock, StraightTransportBlock::continuous()));
            }

            "tp_push_o" => {
                entity_commands.insert((
                    SolidBlock,
                    PushPullBlock::new(PushPullAction::Push, PushPullReach::OneCell),
                ));
            }

            "tp_push_a" => {
                entity_commands.insert((
                    SolidBlock,
                    PushPullBlock::new(PushPullAction::Push, PushPullReach::ToLimit),
                ));
            }

            "tp_pull_o" => {
                entity_commands.insert((
                    SolidBlock,
                    PushPullBlock::new(PushPullAction::Pull, PushPullReach::OneCell),
                ));
            }

            "tp_pull_a" => {
                entity_commands.insert((
                    SolidBlock,
                    PushPullBlock::new(PushPullAction::Pull, PushPullReach::ToLimit),
                ));
            }

            _ => {}
        }

        let entity = entity_commands.id();

        let previous = next_grid_index.insert(position, entity);

        assert!(
            previous.is_none(),
            "ValidatedMap contains duplicate positions"
        );
    }

    grid_index.0 = next_grid_index;
    active_play_world.root = Some(root);
}

#[cfg(test)]
mod grid_index_movement_tests {
    use super::*;

    fn indexed_block() -> (GridIndex, Entity, CurrentGridPosition) {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let current = CurrentGridPosition(GridPosition::new(2, 2));

        let mut index = GridIndex::default();
        index.0.insert(current.0, entity);

        (index, entity, current)
    }

    #[test]
    fn relocation_updates_occupancy_and_current_position() {
        let (mut index, entity, mut current) = indexed_block();
        let source = current.0;
        let destination = GridPosition::new(3, 2);

        assert!(index.try_relocate(entity, &mut current, destination));

        assert_eq!(index.entity_at(source), None);
        assert_eq!(index.entity_at(destination), Some(entity));
        assert_eq!(current.0, destination);
        assert_eq!(index.len(), 1);

        // 다음 이동은 갱신된 논리 좌표에서 출발해야 합니다.
        let next = GridPosition::new(4, 2);

        assert!(index.try_relocate(entity, &mut current, next));

        assert_eq!(index.entity_at(destination), None);
        assert_eq!(index.entity_at(next), Some(entity));
        assert_eq!(current.0, next);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn occupied_destination_preserves_both_blocks() {
        let mut world = World::new();
        let mover = world.spawn_empty().id();
        let blocker = world.spawn_empty().id();

        let source = GridPosition::new(2, 2);
        let destination = GridPosition::new(3, 2);
        let mut current = CurrentGridPosition(source);

        let mut index = GridIndex::default();
        index.0.insert(source, mover);
        index.0.insert(destination, blocker);

        assert!(!index.try_relocate(mover, &mut current, destination));

        assert_eq!(index.entity_at(source), Some(mover));
        assert_eq!(index.entity_at(destination), Some(blocker));
        assert_eq!(current.0, source);
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn stale_source_cannot_move_another_blocks_entry() {
        let mut world = World::new();
        let mover = world.spawn_empty().id();
        let other = world.spawn_empty().id();

        let actual_source = GridPosition::new(2, 2);
        let stale_source = GridPosition::new(3, 2);
        let destination = GridPosition::new(4, 2);
        let mut stale_current = CurrentGridPosition(stale_source);

        let mut index = GridIndex::default();
        index.0.insert(actual_source, mover);
        index.0.insert(stale_source, other);

        assert!(!index.try_relocate(mover, &mut stale_current, destination,));

        assert_eq!(index.entity_at(actual_source), Some(mover));
        assert_eq!(index.entity_at(stale_source), Some(other));
        assert_eq!(index.entity_at(destination), None);
        assert_eq!(stale_current.0, stale_source);
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn relocating_to_the_same_cell_preserves_the_entry() {
        let (mut index, entity, mut current) = indexed_block();
        let destination = current.0;

        assert!(index.try_relocate(entity, &mut current, destination));

        assert_eq!(index.entity_at(destination), Some(entity));
        assert_eq!(current.0, destination);
        assert_eq!(index.len(), 1);
    }
}
