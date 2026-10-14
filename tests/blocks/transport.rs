use crate::common::load_validated_map;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::{CardinalDirection, GridPosition},
    gameplay::{
        ActivePlayWorld, CurrentGridPosition, GameplayPhysicsPlugin, GridIndex, MapSpawnPlugin,
        OriginGridPosition, PHYSICS_HZ, PLAYER_GRAVITY_SCALE, PlayRestartPlugin, PlaySessionPlugin,
        PlayWorld, RestartPlayWorld, SpawnValidatedMap, StraightTransportPlugin,
    },
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BASE_MAP: &str = include_str!("../../assets/maps/synthetic_minimal_map.json");

const START: GridPosition = GridPosition::new(5, 3);
const BALL_START: GridPosition = GridPosition::new(5, 6);

fn app_with_transport(block_id: &str, direction: CardinalDirection, with_stop_block: bool) -> App {
    let mut json: serde_json::Value = serde_json::from_str(BASE_MAP).unwrap();

    let mut blocks = vec![
        serde_json::json!({
            "x": 5,
            "y": 6,
            "block": {
                "type": "item",
                "name": "ball",
                "dir": 0
            }
        }),
        serde_json::json!({
            "x": 20,
            "y": 10,
            "block": {
                "type": "item",
                "name": "star",
                "dir": 0
            }
        }),
        serde_json::json!({
            "x": 5,
            "y": 3,
            "block": {
                "type": "transport",
                "name": block_id,
                "dir": direction.index()
            }
        }),
    ];

    if with_stop_block {
        blocks.push(serde_json::json!({
            "x": 10,
            "y": 3,
            "block": {
                "type": "block",
                "name": "b_normal",
                "dir": 0
            }
        }));
    }

    json["blocks"] = serde_json::json!(blocks);

    let map = load_validated_map(&serde_json::to_string(&json).unwrap());

    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        StraightTransportPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut().write_message(SpawnValidatedMap(map));

    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

fn entity_at(app: &App, grid: GridPosition) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(grid)
        .expect("expected an indexed entity")
}

fn current_grid(app: &App, entity: Entity) -> GridPosition {
    app.world().get::<CurrentGridPosition>(entity).unwrap().0
}

fn set_ball(app: &mut App, entity: Entity, position: Vec2, velocity: Vec2, gravity_scale: f32) {
    app.world_mut().entity_mut(entity).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(velocity),
        GravityScale(gravity_scale),
    ));
}

fn park_ball(app: &mut App, entity: Entity) {
    // 再接触を防ぎ、ブロック自身の動作だけを観察します。
    set_ball(app, entity, Vec2::new(20.0, 12.0), Vec2::ZERO, 0.0);
}

fn run_frames(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn wait_for_cell(app: &mut App, entity: Entity, expected: GridPosition) {
    for _ in 0..120 {
        app.update();

        if current_grid(app, entity) == expected {
            return;
        }
    }

    panic!(
        "transport never reached {:?}; current cell is {:?}",
        expected,
        current_grid(app, entity),
    );
}

fn assert_at_rest(app: &App, entity: Entity, expected: GridPosition) {
    assert_eq!(current_grid(app, entity), expected);

    let expected_world = Vec2::new(expected.x as f32, expected.y as f32);

    let position = app.world().get::<Position>(entity).unwrap().0;

    let transform = app
        .world()
        .get::<Transform>(entity)
        .unwrap()
        .translation
        .truncate();

    assert!(
        position.distance(expected_world) < 0.001,
        "physics position: {:?}, expected: {:?}",
        position,
        expected_world,
    );

    assert!(
        transform.distance(expected_world) < 0.001,
        "transform: {:?}, expected: {:?}",
        transform,
        expected_world,
    );

    assert!(app.world().get::<Collider>(entity).is_some());
}

#[test]
fn one_step_responds_to_real_landings_in_all_four_directions() {
    for direction in CardinalDirection::ALL {
        let mut app = app_with_transport("tp_st_o", direction, false);

        let block = entity_at(&app, START);
        let ball = entity_at(&app, BALL_START);

        let first_destination = START.offset(direction);

        wait_for_cell(&mut app, block, first_destination);

        park_ball(&mut app, ball);
        run_frames(&mut app, 40);

        // 接触が終了した後、自動で次の一歩に進んではいけません。
        assert_at_rest(&app, block, first_destination);

        assert_eq!(
            app.world().get::<OriginGridPosition>(block).unwrap().0,
            START,
        );

        assert_eq!(app.world().resource::<GridIndex>().entity_at(START), None,);

        // 移動後の場所にもう一度落として、再発動も確認します。
        set_ball(
            &mut app,
            ball,
            Vec2::new(first_destination.x as f32, first_destination.y as f32 + 2.0),
            Vec2::ZERO,
            PLAYER_GRAVITY_SCALE,
        );

        let second_destination = first_destination.offset(direction);

        wait_for_cell(&mut app, block, second_destination);

        park_ball(&mut app, ball);
        run_frames(&mut app, 40);

        assert_at_rest(&app, block, second_destination);

        assert_eq!(
            app.world()
                .resource::<GridIndex>()
                .entity_at(first_destination),
            None,
        );
    }
}

#[test]
fn continuous_transport_keeps_moving_after_ball_leaves_and_stops_at_blocker() {
    let mut app = app_with_transport("tp_st_a", CardinalDirection::Right, true);

    let block = entity_at(&app, START);
    let ball = entity_at(&app, BALL_START);
    let blocker = entity_at(&app, GridPosition::new(10, 3));

    wait_for_cell(&mut app, block, GridPosition::new(6, 3));

    park_ball(&mut app, ball);
    run_frames(&mut app, 200);

    assert_at_rest(&app, block, GridPosition::new(9, 3));

    let index = app.world().resource::<GridIndex>();

    assert_eq!(index.entity_at(GridPosition::new(9, 3)), Some(block),);

    assert_eq!(index.entity_at(GridPosition::new(10, 3)), Some(blocker),);

    for x in 5..9 {
        assert_eq!(index.entity_at(GridPosition::new(x, 3)), None,);
    }
}

#[test]
fn side_contact_bounces_ball_without_triggering_transport() {
    let mut app = app_with_transport("tp_st_o", CardinalDirection::Right, false);

    let block = entity_at(&app, START);
    let ball = entity_at(&app, BALL_START);

    set_ball(
        &mut app,
        ball,
        Vec2::new(3.0, 3.0),
        Vec2::new(5.0, 0.0),
        0.0,
    );

    let mut bounced = false;

    for _ in 0..60 {
        app.update();

        let velocity = app.world().get::<LinearVelocity>(ball).unwrap().0;

        bounced |= velocity.x < -0.1;

        assert_eq!(current_grid(&app, block), START);
    }

    assert!(bounced, "ball never actually hit the side of the block");
    assert_at_rest(&app, block, START);
}

#[test]
fn restart_during_motion_restores_original_cell_and_clears_continuation() {
    let mut app = app_with_transport("tp_st_a", CardinalDirection::Right, true);

    let old_root = app.world().resource::<ActivePlayWorld>().root().unwrap();

    let original_map = app
        .world()
        .get::<PlayWorld>(old_root)
        .unwrap()
        .definition()
        .clone();

    let old_block = entity_at(&app, START);
    let old_ball = entity_at(&app, BALL_START);

    wait_for_cell(&mut app, old_block, GridPosition::new(6, 3));

    park_ball(&mut app, old_ball);
    run_frames(&mut app, 5);

    let moving_x = app.world().get::<Position>(old_block).unwrap().x;

    assert!(
        moving_x > 5.0 && moving_x < 6.0,
        "restart must be tested during animation: x={moving_x}",
    );

    app.world_mut().write_message(RestartPlayWorld);
    app.update();

    let new_root = app.world().resource::<ActivePlayWorld>().root().unwrap();

    assert_ne!(new_root, old_root);
    assert!(!app.world().entities().contains(old_root));
    assert!(!app.world().entities().contains(old_block));
    assert!(!app.world().entities().contains(old_ball));

    assert_eq!(
        app.world().get::<PlayWorld>(new_root).unwrap().definition(),
        &original_map,
    );

    let new_block = entity_at(&app, START);
    let new_ball = entity_at(&app, BALL_START);

    park_ball(&mut app, new_ball);
    run_frames(&mut app, 100);

    // 再スタート前の連続移動が新しいブロックで再開しないこと。
    assert_at_rest(&app, new_block, START);

    assert_eq!(
        app.world()
            .resource::<GridIndex>()
            .entity_at(GridPosition::new(6, 3)),
        None,
    );
}
