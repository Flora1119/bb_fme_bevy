use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{CardinalDirection, GridPosition, ValidatedMap},
    gameplay::{
        ActivePlayWorld, BlockFacing, BlockIdentity, BlockOptions, CollectibleStar,
        CurrentGridPosition, DeadlySpike, GridIndex, MapSpawnPlugin, OriginGridPosition, PlayWorld,
        PlayerBall, RuntimeBlock, SolidBlock, SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::prelude::*;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");
const MINIMAL_MAP: &str = include_str!("../assets/maps/synthetic_minimal_map.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(MINIMAL_MAP).expect("map fixture must deserialize");

    ValidatedMap::from_document(&document, &config).expect("fixture must validate")
}

fn app_with_spawned_map() -> (App, ValidatedMap) {
    let map = load_validated_map();
    let mut app = App::new();

    app.add_plugins(MapSpawnPlugin);

    app.world_mut()
        .write_message(SpawnValidatedMap(map.clone()));

    app.update();

    (app, map)
}

#[test]
fn plugin_spawns_one_play_world_and_one_entity_per_block() {
    let (mut app, map) = app_with_spawned_map();

    let root = app
        .world()
        .resource::<ActivePlayWorld>()
        .root()
        .expect("play world root must be active");

    let play_world = app
        .world()
        .get::<PlayWorld>(root)
        .expect("root must contain the immutable map definition");

    assert_eq!(play_world.definition(), &map);

    assert_eq!(
        app.world().get::<Visibility>(root),
        Some(&Visibility::Inherited)
    );

    let world = app.world_mut();
    let mut blocks = world.query_filtered::<Entity, With<RuntimeBlock>>();

    assert_eq!(blocks.iter(world).count(), map.blocks.len());
    assert_eq!(world.resource::<GridIndex>().len(), map.blocks.len());
}

#[test]
fn block_entities_preserve_runtime_data_and_grid_transform() {
    let (app, _) = app_with_spawned_map();

    let root = app
        .world()
        .resource::<ActivePlayWorld>()
        .root()
        .expect("play world root must be active");

    let ball_entity = app
        .world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(2, 2))
        .expect("ball position must be indexed");

    let world = app.world();

    let identity = world
        .get::<BlockIdentity>(ball_entity)
        .expect("block identity must exist");

    assert_eq!(identity.id.as_str(), "ball");

    assert_eq!(
        world.get::<BlockFacing>(ball_entity),
        Some(&BlockFacing(CardinalDirection::Up))
    );

    assert_eq!(
        world.get::<CurrentGridPosition>(ball_entity),
        Some(&CurrentGridPosition(GridPosition::new(2, 2)))
    );

    assert_eq!(
        world.get::<OriginGridPosition>(ball_entity),
        Some(&OriginGridPosition(GridPosition::new(2, 2)))
    );

    assert_eq!(world.get::<BlockOptions>(ball_entity).unwrap().0, vec![]);

    let transform = world
        .get::<Transform>(ball_entity)
        .expect("block transform must exist");

    assert_eq!(transform.translation, Vec3::new(2.0, 2.0, 0.0));

    assert_eq!(
        world.get::<ChildOf>(ball_entity).map(|child_of| child_of.0),
        Some(root)
    );
}

#[test]
fn a_new_spawn_request_replaces_the_previous_play_world() {
    let (mut app, mut replacement) = app_with_spawned_map();

    let previous_root = app
        .world()
        .resource::<ActivePlayWorld>()
        .root()
        .expect("first root must exist");

    let previous_blocks: Vec<Entity> = {
        let world = app.world_mut();
        let mut blocks = world.query_filtered::<Entity, With<RuntimeBlock>>();

        blocks.iter(world).collect()
    };

    replacement.map_name = "replacement".to_owned();

    app.world_mut()
        .write_message(SpawnValidatedMap(replacement.clone()));

    app.update();

    let next_root = app
        .world()
        .resource::<ActivePlayWorld>()
        .root()
        .expect("replacement root must exist");

    assert_ne!(next_root, previous_root);
    assert!(!app.world().entities().contains(previous_root));

    assert!(
        previous_blocks
            .into_iter()
            .all(|entity| !app.world().entities().contains(entity))
    );

    assert_eq!(
        app.world()
            .get::<PlayWorld>(next_root)
            .unwrap()
            .definition(),
        &replacement
    );
}

#[test]
fn minial_slice_blocks_receive_gameplay_roles() {
    let (app, _) = app_with_spawned_map();
    let index = app.world().resource::<GridIndex>();

    let ball = index
        .entity_at(GridPosition::new(2, 2))
        .expect("ball must be indexed");

    let star = index
        .entity_at(GridPosition::new(8, 2))
        .expect("star must be indexed");

    let solid = index
        .entity_at(GridPosition::new(2, 0))
        .expect("normal block must be indexed");

    let spike = index
        .entity_at(GridPosition::new(12, 0))
        .expect("normal spike must be indexed");

    assert!(app.world().get::<PlayerBall>(ball).is_some());
    assert!(app.world().get::<CollectibleStar>(star).is_some());
    assert!(app.world().get::<SolidBlock>(solid).is_some());
    assert!(app.world().get::<DeadlySpike>(spike).is_some());

    assert!(app.world().get::<SolidBlock>(ball).is_none());
    assert!(app.world().get::<PlayerBall>(star).is_none());
    assert!(app.world().get::<DeadlySpike>(solid).is_none());
    assert!(app.world().get::<SolidBlock>(spike).is_none());
}
