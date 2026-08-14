use bb_fme_bevy::{
    block::{BlockAssetConfig, BlockCategory},
    domain::{CardinalDirection, GridPosition, ValidatedMap},
    map::{
        MapBlock, MapBlockEntry, MapBlockOption, MapBlockOptionValue, MapDocument,
        ValidatedMapBuildError,
    },
};

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

fn load_config() -> BlockAssetConfig {
    serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize")
}

fn load_map() -> MapDocument {
    serde_json::from_str(MINIMAL_MAP).expect("map fixture must deserialize")
}

fn push_block(
    document: &mut MapDocument,
    x: i32,
    y: i32,
    category: &str,
    name: &str,
    direction: i32,
) {
    document.blocks.push(MapBlockEntry {
        x,
        y,
        block: MapBlock {
            r#type: category.to_owned(),
            name: name.to_owned(),
            dir: direction,
            extra: Default::default(),
        },
        extra: Default::default(),
    });
}

#[test]
fn document_projects_to_typed_runtime_map() {
    let config = load_config();
    let document = load_map();

    let runtime =
        ValidatedMap::from_document(&document, &config).expect("valid document must project");

    assert_eq!(runtime.map_name, "synthetic_minimal_map");
    assert_eq!(runtime.settings.required_stars, 1);
    assert_eq!(runtime.settings.size.width, 25);
    assert_eq!(runtime.settings.size.height, 15);
    assert_eq!(runtime.blocks.len(), 3);

    let ball = runtime
        .block_at(GridPosition::new(2, 2))
        .expect("ball must exist");

    assert_eq!(ball.id.as_str(), "ball");
    assert_eq!(ball.category, BlockCategory::Item);
    assert_eq!(ball.direction, CardinalDirection::Up);
}

#[test]
fn missing_options_use_config_defaults() {
    let config = load_config();
    let document = load_map();

    let runtime =
        ValidatedMap::from_document(&document, &config).expect("valid document must project");

    let star = runtime
        .block_at(GridPosition::new(8, 2))
        .expect("star must exist");

    assert_eq!(star.options.len(), 1);
    assert_eq!(star.options[0].name, "Scale");
    assert_eq!(star.options[0].value, 1.0);
}

#[test]
fn explicit_options_override_defaults() {
    let config = load_config();
    let mut document = load_map();

    document.block_options = Some(vec![MapBlockOption {
        x: 8,
        y: 2,
        name: "star".to_owned(),
        options: vec![MapBlockOptionValue {
            value_name: "Scale".to_owned(),
            value: 0.75,
            extra: Default::default(),
        }],
        extra: Default::default(),
    }]);

    let runtime = ValidatedMap::from_document(&document, &config)
        .expect("valid explicit options must project");

    let star = runtime
        .block_at(GridPosition::new(8, 2))
        .expect("star must exist");

    assert_eq!(star.options[0].value, 0.75);
}

#[test]
fn special_positions_become_optional_grid_positions() {
    let config = load_config();
    let mut document = load_map();

    document.map_settings.tp1_exit.x = 6;
    document.map_settings.tp1_exit.y = 4;

    push_block(&mut document, 6, 4, "funcblock", "fb_tp1_out", 0);

    document.map_settings.portal1_positions.a_px = 10;
    document.map_settings.portal1_positions.a_py = 4;
    document.map_settings.portal1_positions.b_px = 15;
    document.map_settings.portal1_positions.b_py = 8;

    push_block(&mut document, 10, 4, "funcblock", "fb_portal1", 0);

    push_block(&mut document, 15, 8, "funcblock", "fb_portal1", 1);

    let runtime =
        ValidatedMap::from_document(&document, &config).expect("special positions must project");

    assert_eq!(
        runtime.settings.teleport_1_exit,
        Some(GridPosition::new(6, 4))
    );

    assert_eq!(runtime.settings.teleport_2_exit, None);

    assert_eq!(runtime.settings.portal_1.a, Some(GridPosition::new(10, 4)));

    assert_eq!(runtime.settings.portal_1.b, Some(GridPosition::new(15, 8)));
}

#[test]
fn invalid_document_cannot_be_projected() {
    let config = load_config();
    let mut document = load_map();

    document.blocks[1].block.name = "unknown_test_block".to_owned();

    let error = ValidatedMap::from_document(&document, &config)
        .expect_err("invalid document must not project");

    match error {
        ValidatedMapBuildError::Validation(errors) => {
            assert!(
                errors
                    .problems()
                    .iter()
                    .any(|problem| { problem.contains("unknown block ID",) })
            );
        }
        ValidatedMapBuildError::ProjectionInvariant(message) => {
            panic!("expected validation error, got invariant error: {message}");
        }
    }
}
