use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    map::MapDocument,
};
use serde_json::Value;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

const UNITY_GOLDEN_MAP: &str = include_str!("../assets/maps/unity_phase4_vertical_slice.json");

#[test]
fn unity_map_json_round_trips_without_data_loss() {
    let original: Value = serde_json::from_str(MINIMAL_MAP).expect("fixture must be valid JSON");

    let document: MapDocument =
        serde_json::from_value(original.clone()).expect("Unity map JSON must derserialize");

    assert_eq!(document.map_name, "synthetic_minimal_map");
    assert_eq!(document.author, "Guest");
    assert_eq!(document.map_settings.size.width, 25);
    assert_eq!(document.map_settings.size.height, 15);
    assert_eq!(document.map_settings.tp1_exit.x, -1);
    assert_eq!(document.blocks.len(), 4);

    assert_eq!(
        document.extra.get("future_top_level_field"),
        Some(&Value::String("preserve me".to_owned()))
    );

    assert_eq!(
        document.map_settings.extra.get("future_setting"),
        Some(&Value::Bool(true))
    );

    let serialized = serde_json::to_value(&document).expect("map document must serialize");

    assert_eq!(serialized, original);
}

#[test]
fn block_options_may_be_null() {
    let json = MINIMAL_MAP.replace("\"block_options\": []", "\"block_options\": null");

    let document: MapDocument =
        serde_json::from_str(&json).expect("null block_options must be accepted");

    assert!(document.block_options.is_none());
}

#[test]
fn unity_editor_golden_fixture_round_trips_and_validates() {
    let original: Value =
        serde_json::from_str(UNITY_GOLDEN_MAP).expect("Unity golden fixture must be valid JSON");

    let document: MapDocument =
        serde_json::from_value(original.clone()).expect("Unity golden fixture must deserialize");

    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let validated = ValidatedMap::from_document(&document, &config)
        .expect("Unity golden fixture must validate");

    assert_eq!(document.map_name, "editor_map");
    assert!(!document.author.trim().is_empty());

    assert_eq!(document.map_settings.size.width, 25);
    assert_eq!(document.map_settings.size.height, 15);
    assert_eq!(document.map_settings.star_count, 1);

    assert_eq!(document.blocks.len(), 5);

    assert!(
        document.blocks.iter().any(|entry| {
            entry.x == 2
                && entry.y == 2
                && entry.block.name == "ball"
                && entry.block.r#type == "item"
                && entry.block.dir == 0
        }),
        "Unity fixture must preserve the player ball"
    );

    assert!(
        document.blocks.iter().any(|entry| {
            entry.x == 6
                && entry.y == 2
                && entry.block.name == "star"
                && entry.block.r#type == "item"
                && entry.block.dir == 0
        }),
        "Unity fixture must preserve the collectible star"
    );

    assert!(
        document.blocks.iter().any(|entry| {
            entry.x == 2
                && entry.y == 0
                && entry.block.name == "b_normal"
                && entry.block.r#type == "block"
                && entry.block.dir == 0
        }),
        "Unity fixture must preserve the normal block"
    );

    assert!(
        document.blocks.iter().any(|entry| {
            entry.x == 8
                && entry.y == 0
                && entry.block.name == "s_normal"
                && entry.block.r#type == "spike"
                && entry.block.dir == 1
        }),
        "Unity fixture must preserve spike direction"
    );

    assert!(
        document.blocks.iter().any(|entry| {
            entry.x == 10
                && entry.y == 0
                && entry.block.name == "fb_jump"
                && entry.block.r#type == "funcblock"
                && entry.block.dir == 0
        }),
        "Unity fixture must preserve the jump block"
    );

    let block_options = document
        .block_options
        .as_deref()
        .expect("Unity fixture must contain block options");

    assert_eq!(block_options.len(), 1);

    let star_options = block_options
        .iter()
        .find(|entry| entry.x == 6 && entry.y == 2 && entry.name == "star")
        .expect("star options must be preserved");

    assert_eq!(star_options.options.len(), 1);
    assert_eq!(star_options.options[0].value_name, "Scale");

    assert!(
        (star_options.options[0].value - 0.7).abs() <= 0.000_001,
        "star Scale option must remain 0.7"
    );

    assert_eq!(validated.settings.size.width, 25);
    assert_eq!(validated.settings.size.height, 15);
    assert_eq!(validated.settings.required_stars, 1);

    let validated_star = validated
        .block_at(GridPosition::new(6, 2))
        .expect("validated map must contain the star");

    assert_eq!(validated_star.id.as_str(), "star");
    assert_eq!(validated_star.options.len(), 1);
    assert_eq!(validated_star.options[0].name, "Scale");

    assert!((validated_star.options[0].value - 0.7).abs() <= 0.000_001);

    let serialized = serde_json::to_string(&document).expect("golden map document must serialize");

    let round_tripped: MapDocument =
        serde_json::from_str(&serialized).expect("serialized golden map must deserialize again");

    assert_eq!(
        round_tripped, document,
        "Unity map data must survive a Rust deserialize/serialize round trip"
    );
}
