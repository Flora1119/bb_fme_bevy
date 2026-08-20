use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{GridPosition, ValidatedMap},
    gameplay::{
        GameplayPhysicsPlugin, GridIndex, MapBoundary, MapBoundaryPlugin, MapSpawnPlugin,
        PHYSICS_HZ, PlayRestartPlugin, PlaySession, PlaySessionPlugin, PlaySessionState,
        RestartPlayWorld, SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const BOUNDARY_MAP: &str = include_str!("../assets/maps/phase8_jump_boundary_sandbox.json");

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(BOUNDARY_MAP).expect("boundary map must deserialize");

    ValidatedMap::from_document(&document, &config).expect("boundary map must validate")
}

fn app_with_boundaries() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        MapBoundaryPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    app.finish();
    app.cleanup();

    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

fn player(app: &App) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(4, 2))
        .expect("player must be indexed")
}

fn move_player(app: &mut App, entity: Entity, position: Vec2) {
    app.world_mut().entity_mut(entity).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(Vec2::ZERO),
        GravityScale(0.0),
    ));
}

#[test]
fn unity_style_map_boundaries_are_spawned() {
    let mut app = app_with_boundaries();

    let world = app.world_mut();

    let mut boundaries = world.query_filtered::<Entity, (With<MapBoundary>, With<Sensor>)>();

    assert_eq!(boundaries.iter(world).count(), 4);
}

#[test]
fn touching_any_map_boundary_marks_the_session_dead() {
    // phase8 map은 25 x 15입니다.
    let boundary_centers = [
        Vec2::new(-2.0, 7.5),
        Vec2::new(27.0, 7.5),
        Vec2::new(12.5, 17.0),
        Vec2::new(12.5, -2.0),
    ];

    for position in boundary_centers {
        let mut app = app_with_boundaries();

        let ball = player(&app);

        move_player(&mut app, ball, position);

        for _ in 0..4 {
            app.update();

            if app.world().resource::<PlaySession>().state() == PlaySessionState::Dead {
                break;
            }
        }

        assert_eq!(
            app.world().resource::<PlaySession>().state(),
            PlaySessionState::Dead,
            "boundary at {position:?} did not kill player"
        );
    }
}

#[test]
fn boundary_death_can_be_restarted_cleanly() {
    let mut app = app_with_boundaries();

    let old_player = player(&app);

    move_player(&mut app, old_player, Vec2::new(12.5, -2.0));

    for _ in 0..4 {
        app.update();

        if app.world().resource::<PlaySession>().state() == PlaySessionState::Dead {
            break;
        }
    }

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Dead
    );

    app.world_mut().write_message(RestartPlayWorld);

    app.update();

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing
    );

    assert!(!app.world().entities().contains(old_player));

    let new_player = player(&app);

    assert_ne!(new_player, old_player);

    let transform = app
        .world()
        .get::<Transform>(new_player)
        .expect("new player must have Transform");

    assert_eq!(transform.translation, Vec3::new(4.0, 2.0, 0.0));
}
