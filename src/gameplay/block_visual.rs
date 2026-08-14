use super::{BLOCK_WORLD_SIZE, BlockIdentity, MapSpawnSet, RuntimeBlock};
use crate::block::{BlockCategory, BlockId};
use bevy::prelude::*;
use std::collections::HashMap;

type NewUnvisualizedRuntimeBlock = (Added<RuntimeBlock>, Without<Sprite>);

pub fn catalog_visual_path(category: BlockCategory, block_id: &str) -> String {
    format!("sprites/{category}/{block_id}.png")
}

pub struct BlockVisualPlugin;

impl Plugin for BlockVisualPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlockVisualRegistry>().add_systems(
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

#[derive(Resource, Debug, Default)]
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

    fn load_for(&mut self, identity: &BlockIdentity, asset_server: &AssetServer) -> BlockVisual {
        if let Some(visual) = self.visuals.get(&identity.id) {
            return visual.clone();
        }

        let visual = BlockVisual {
            image: asset_server.load(catalog_visual_path(identity.category, identity.id.as_str())),
            size: Vec2::splat(BLOCK_WORLD_SIZE),
        };

        self.visuals.insert(identity.id.clone(), visual.clone());
        visual
    }
}

fn attach_registered_block_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut registry: ResMut<BlockVisualRegistry>,
    blocks: Query<(Entity, &BlockIdentity), NewUnvisualizedRuntimeBlock>,
) {
    for (entity, identity) in &blocks {
        let visual = registry.load_for(identity, &asset_server);

        commands.entity(entity).insert((
            RegisteredBlockVisual,
            Sprite {
                image: visual.image,
                custom_size: Some(visual.size),
                ..default()
            },
        ));
    }
}
