use bb_fme_bevy::map::MapDocument;
use serde_json::Value;

const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

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
