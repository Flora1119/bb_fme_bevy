use bb_fme_bevy::{
    block::BlockAssetConfig,
    map::{MapDocument, validate_map_document},
};
use serde_json::json;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const MINIMAL_MAP: &str = include_str!("fixtures/synthetic_minimal_map.json");

fn load_config() -> BlockAssetConfig {
    serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize")
}

fn load_map() -> MapDocument {
    serde_json::from_str(MINIMAL_MAP).expect("map fixture must deserialize")
}

#[test]
fn valid_map_passes_validation() {
    let config = load_config();
    let document = load_map();

    validate_map_document(&document, &config).expect("synthetic map must be valid");
}

#[test]
fn validation_reports_multiple_block_problems() {
    let config = load_config();
    let mut document = load_map();

    document.blocks[1].block.name = "unknown_test_block".to_owned();

    document.blocks[1].block.r#type = "obstacle".to_owned();

    document.blocks[2].x = document.blocks[0].x;
    document.blocks[2].y = document.blocks[0].y;

    document.map_settings.star_count = 99;

    let errors =
        validate_map_document(&document, &config).expect_err("invalid map must fail validation");

    let message = errors.to_string();

    assert!(message.contains("unknown block ID"));
    assert!(message.contains("occupy the same position"));
    assert!(message.contains("declare 99 stars"));
}

#[test]
fn validation_requires_exactly_one_ball() {
    let config = load_config();
    let mut document = load_map();

    document.blocks.retain(|entry| entry.block.name != "ball");

    let errors =
        validate_map_document(&document, &config).expect_err("map without a ball must fail");

    assert!(
        errors
            .problems()
            .iter()
            .any(|problem| { problem.contains("exactly one ball") })
    );
}

#[test]
fn validation_rejects_out_of_bounds_block() {
    let config = load_config();
    let mut document = load_map();

    document.blocks[0].x = document.map_settings.size.width;

    let errors =
        validate_map_document(&document, &config).expect_err("out-of-bounds block must fail");

    assert!(
        errors
            .problems()
            .iter()
            .any(|problem| { problem.contains("outside map size") })
    );
}

#[test]
fn validation_checks_option_values() {
    let config = load_config();

    let mut json_document: serde_json::Value =
        serde_json::from_str(MINIMAL_MAP).expect("fixture must be valid JSON");

    json_document["blocks"]
        .as_array_mut()
        .expect("blocks must be an array")
        .push(json!({
            "x": 10,
            "y": 2,
            "block": {
                "type": "obstacle",
                "name": "ob_cannon",
                "dir": 0
            }
        }));

    json_document["block_options"] = json!([
        {
            "x": 10,
            "y": 2,
            "name": "ob_cannon",
            "options": [
                {
                    "value_name": "Delay",
                    "value": 0.5
                },
                {
                    "value_name": "Interval",
                    "value": 1.5
                },
                {
                    "value_name": "Value",
                    "value": 999.0
                }
            ]
        }
    ]);

    let document: MapDocument =
        serde_json::from_value(json_document).expect("modified fixture must deserialize");

    let errors =
        validate_map_document(&document, &config).expect_err("invalid option value must fail");

    assert!(
        errors
            .problems()
            .iter()
            .any(|problem| { problem.contains("outside 0..=15") })
    );
}
