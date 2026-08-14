use super::{BLOCK_WORLD_SIZE, BlockIdentity, MapSpawnSet, RuntimeBlock};
use crate::block::BlockId;
use bevy::prelude::*;
use std::collections::HashMap;

const POC_VISUAL_PATHS: [(&str, &str); 3] = [
    ("ball", "sprites/item/ball.png"),
    ("star", "sprites/item/star.png"),
    ("b_normal", "sprites/block/b_normal.png"),
];

type NewUnvisualizedRuntimeBlock = (Added<RuntimeBlock>, Without<Sprite>);

pub struct BlockVisualPlugin;

impl Plugin for BlockVisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_block_visual_registry)
            .add_systems(
                Update,
                attach_registered_block_visuals
                    .in_set(BlockVisualSet)
                    .after(MapSpawnSet),
            );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockVisualSet;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisteredBlockVisual;

#[derive(Debug, Clone)]
pub struct BlockVisual {
    image: Handle<Image>,
    size: Vec2,
}

impl BlockVisual {
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub const fn size(&self) -> Vec2 {
        self.size
    }
}

#[derive(Resource, Debug)]
pub struct BlockVisualRegistry {
    visuals: HashMap<BlockId, BlockVisual>,
}

impl BlockVisualRegistry {
    pub fn get(&self, block_id: &str) -> Option<&BlockVisual> {
        self.visuals.get(block_id)
    }

    pub fn len(&self) -> usize {
        self.visuals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visuals.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&BlockId, &BlockVisual)> {
        self.visuals.iter()
    }
}

fn load_block_visual_registry(mut commands: Commands, asset_server: Res<AssetServer>) {
    let visuals = POC_VISUAL_PATHS
        .into_iter()
        .map(|(block_id, path)| {
            (
                BlockId::from(block_id),
                BlockVisual {
                    image: asset_server.load(path),
                    size: Vec2::splat(BLOCK_WORLD_SIZE),
                },
            )
        })
        .collect();

    commands.insert_resource(BlockVisualRegistry { visuals });
}

fn attach_registered_block_visuals(
    mut commands: Commands,
    registry: Res<BlockVisualRegistry>,
    blocks: Query<(Entity, &BlockIdentity), NewUnvisualizedRuntimeBlock>,
) {
    for (entity, identity) in &blocks {
        let Some(visual) = registry.get(identity.id.as_str()) else {
            continue;
        };

        commands.entity(entity).insert((
            RegisteredBlockVisual,
            Sprite {
                image: visual.image().clone(),
                custom_size: Some(visual.size()),
                ..default()
            },
        ));
    }
}
