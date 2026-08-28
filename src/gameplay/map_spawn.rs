use super::{
    ClockBlock, CollectibleStar, DeadlySpike, JumpBlock, OneShotFunctionBlock, PlayerBall,
    SolidBlock, StraightBlock, TeleportEntrance, TeleportExit, TransparentStar,
    spike_has_solid_collider,
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
