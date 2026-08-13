use bb_fme_bevy::block::{
    BlockAssetConfig, BlockCategory, BlockId, EXPECTED_BLOCK_CATEGORY_COUNT,
    EXPECTED_BLOCK_ID_COUNT,
};

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

fn load_config() -> BlockAssetConfig {
    serde_json::from_str(BLOCK_CONFIG).expect("Unity block asset config must deserialize")
}

#[test]
fn unity_block_config_has_expected_inventory() {
    let config = load_config();

    assert_eq!(config.block_groups.len(), EXPECTED_BLOCK_CATEGORY_COUNT);

    assert_eq!(config.block_id_count(), EXPECTED_BLOCK_ID_COUNT);

    let expected_category_counts = [
        (BlockCategory::Item, 17),
        (BlockCategory::Block, 4),
        (BlockCategory::Spike, 6),
        (BlockCategory::Funcblock, 17),
        (BlockCategory::Switch, 12),
        (BlockCategory::Whiteblock, 6),
        (BlockCategory::Transport, 7),
        (BlockCategory::Laser, 6),
        (BlockCategory::Obstacle, 7),
    ];

    for (category, expected_count) in expected_category_counts {
        let blocks = config
            .blocks_in(category)
            .unwrap_or_else(|| panic!("missing category {category}"));

        assert_eq!(
            blocks.len(),
            expected_count,
            "unexpected number of blocks in {category}"
        );
    }
}

#[test]
fn unity_block_config_passes_validation() {
    let config = load_config();

    config.validate().expect("Unity block config must be valid");
}

#[test]
fn option_definitions_are_loaded() {
    let config = load_config();

    let cannon_options = config
        .block_options
        .get(&BlockId::from("ob_cannon"))
        .expect("ob_cannon options must exist");

    assert_eq!(cannon_options.len(), 3);
    assert_eq!(cannon_options[0].value_name, "Delay");
    assert_eq!(cannon_options[1].value_name, "Interval");
    assert_eq!(cannon_options[2].value_name, "Value");

    let gear_options = config
        .block_options
        .get(&BlockId::from("ob_gear"))
        .expect("ob_gear options must exist");

    assert_eq!(gear_options[0].value_name, "Speed");
    assert_eq!(gear_options[1].value_name, "CounterClockWise");
}

#[test]
fn config_semantically_round_trips() {
    let config = load_config();

    let serialized = serde_json::to_string(&config).expect("block config must serialize");

    let reparsed: BlockAssetConfig =
        serde_json::from_str(&serialized).expect("serialized block config must deserialize");

    assert_eq!(reparsed, config);
}

#[test]
fn validation_rejects_unknown_option_block() {
    let mut config = load_config();

    config
        .option_blocks
        .push(BlockId::from("unknown_test_block"));

    let errors = config
        .validate()
        .expect_err("unknown block must fail validation");

    assert!(
        errors
            .problems()
            .iter()
            .any(|problem| problem.contains("unknown_test_block"))
    );
}
