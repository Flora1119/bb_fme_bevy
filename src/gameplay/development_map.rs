use super::SpawnValidatedMap;
use crate::{block::BlockAssetConfig, domain::ValidatedMap, map::MapDocument};
use bevy::prelude::*;

const BLOCK_CONFIG: &str = include_str!("../../assets/config/block_assets_config.json");

const DEVELOPMENT_MAP: &str = include_str!("../../assets/maps/phase4_playthrough.json");

pub struct DevelopmentMapPlugin;

impl Plugin for DevelopmentMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, queue_development_map);
    }
}

fn queue_development_map(mut requests: MessageWriter<SpawnValidatedMap>) {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("embedded block config must deserialize");

    config
        .validate()
        .expect("embedded block config must validate");

    let document: MapDocument =
        serde_json::from_str(DEVELOPMENT_MAP).expect("embedded development map must deserialize");

    let map = ValidatedMap::from_document(&document, &config)
        .expect("embedded development map must validate");

    requests.write(SpawnValidatedMap(map));
}
