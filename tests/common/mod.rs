use bb_fme_bevy::{block::BlockAssetConfig, domain::ValidatedMap, map::MapDocument};

const BLOCK_CONFIG: &str = include_str!("../../assets/config/block_assets_config.json");

pub fn load_validated_map(map_json: &str) -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument = serde_json::from_str(map_json).expect("test map must deserialize");

    ValidatedMap::from_document(&document, &config).expect("test map must validate")
}
