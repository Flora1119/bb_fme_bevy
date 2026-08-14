use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        BlockVisualPlugin, BlockVisualRegistry, GridIndex, MapPresentationPlugin, MapSpawnPlugin,
        PlaceholderVisual, RegisteredBlockVisual, SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::{
    asset::AssetApp,
    image::{CompressedImageFormats, ImageLoader},
    prelude::*,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");
const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(MINIMAL_MAP).expect("map fixture must deserialize");

    ValidatedMap::from_document(&document, &config).expect("fixture must validate")
}

fn app_with_block_visuals() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..default()
        },
        ImagePlugin::default_nearest(),
        MapSpawnPlugin,
        BlockVisualPlugin,
        MapPresentationPlugin,
    ));
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    wait_until_images_are_loaded(&mut app);

    app
}

fn wait_until_images_are_loaded(app: &mut App) {
    for _ in 0..10_000 {
        app.update();

        let world = app.world();
        let Some(registry) = world.get_resource::<BlockVisualRegistry>() else {
            continue;
        };
        let asset_server = world.resource::<AssetServer>();

        if registry
            .iter()
            .all(|(_, visual)| asset_server.is_loaded_with_dependencies(visual.image()))
        {
            return;
        }

        std::thread::sleep(Duration::from_millis(1));
    }

    let world = app.world();
    let registry = world.resource::<BlockVisualRegistry>();
    let asset_server = world.resource::<AssetServer>();
    let states: Vec<_> = registry
        .iter()
        .map(|(block_id, visual)| {
            (
                block_id.to_string(),
                asset_server.get_load_state(visual.image()),
            )
        })
        .collect();

    panic!("timed out while loading block PNG assets: {states:?}");
}

#[test]
fn registry_loads_the_three_unity_png_files() {
    let app = app_with_block_visuals();
    let registry = app.world().resource::<BlockVisualRegistry>();
    let images = app.world().resource::<Assets<Image>>();

    assert_eq!(registry.len(), 3);

    for block_id in ["ball", "star", "b_normal"] {
        let visual = registry
            .get(block_id)
            .expect("PoC block must have a registered visual");
        let image = images
            .get(visual.image())
            .expect("registered PNG must be loaded as an Image asset");

        assert_eq!(image.size(), UVec2::splat(64));
        assert_eq!(visual.size(), Vec2::ONE);
    }
}

#[test]
fn registered_sprites_replace_the_placeholder_visuals() {
    let app = app_with_block_visuals();
    let registry = app.world().resource::<BlockVisualRegistry>();
    let index = app.world().resource::<GridIndex>();

    let cases = [
        (GridPosition::new(2, 2), "ball"),
        (GridPosition::new(8, 2), "star"),
        (GridPosition::new(5, 0), "b_normal"),
    ];

    for (position, block_id) in cases {
        let entity = index
            .entity_at(position)
            .expect("fixture block must be indexed");
        let visual = registry
            .get(block_id)
            .expect("fixture block must be registered");
        let sprite = app
            .world()
            .get::<Sprite>(entity)
            .expect("registered block must receive a Sprite");

        assert_eq!(&sprite.image, visual.image());
        assert_eq!(sprite.custom_size, Some(Vec2::ONE));
        assert!(app.world().get::<RegisteredBlockVisual>(entity).is_some());
        assert!(app.world().get::<PlaceholderVisual>(entity).is_none());
    }
}
