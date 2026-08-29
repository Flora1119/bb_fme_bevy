mod common;
use avian2d::prelude::*;
use bb_fme_bevy::{
    domain::GridPosition,
    gameplay::{
        BlockPhysicsBody, DeadlySpike, GameplayPhysicsPlugin, GridIndex, MapSpawnPlugin,
        SolidBlock, SolidColliderChild, SpawnValidatedMap, SpikeSensorCollider,
    },
};
use bevy::{asset::Assets, gizmos::GizmoAsset, prelude::*, transform::TransformPlugin};
use common::load_validated_map;

const SPIKE_MAP: &str = r#"
{
    "map_name": "spike_collider_profiles",
    "author": "Development",
    "map_settings": {
        "time_limit": 60.0,
        "show_time_ranking": false,
        "star_count": 1,
        "size": {
            "width": 25,
            "height": 15
        },
        "tp1_exit": {
            "x": -1,
            "y": -1
        },
        "tp2_exit": {
            "x": -1,
            "y": -1
        },
        "portal1_positions": {
            "a_px": -1,
            "a_py": -1,
            "b_px": -1,
            "b_py": -1
        },
        "portal2_positions": {
            "a_px": -1,
            "a_py": -1,
            "b_px": -1,
            "b_py": -1
        },
        "sw_el": true,
        "sw_b1": true,
        "sw_b2": true
    },
    "blocks": [
        {
            "x": 2,
            "y": 4,
            "block": {
                "type": "item",
                "name": "ball",
                "dir": 0
            }
        },
        {
            "x": 23,
            "y": 13,
            "block": {
                "type": "item",
                "name": "star",
                "dir": 0
            }
        },
        {
            "x": 4,
            "y": 0,
            "block": {
                "type": "spike",
                "name": "s_normal",
                "dir": 0
            }
        },
        {
            "x": 7,
            "y": 0,
            "block": {
                "type": "spike",
                "name": "s_half",
                "dir": 0
            }
        },
        {
            "x": 10,
            "y": 0,
            "block": {
                "type": "spike",
                "name": "s_b_normal",
                "dir": 0
            }
        },
        {
            "x": 13,
            "y": 0,
            "block": {
                "type": "spike",
                "name": "s_b_two",
                "dir": 0
            }
        },
        {
            "x": 16,
            "y": 0,
            "block": {
                "type": "spike",
                "name": "s_b_o_half",
                "dir": 0
            }
        },
        {
            "x": 19,
            "y": 0,
            "block": {
                "type": "spike",
                "name": "s_b_o_two",
                "dir": 0
            }
        }
    ],
    "block_options": []
}
"#;

#[derive(Debug, Clone, Copy)]
struct RectSnapshot {
    entity: Entity,
    size: Vec2,
    offset: Vec2,
}

fn app_with_spikes() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map(SPIKE_MAP)));

    app.finish();
    app.cleanup();

    app.update();
    app.update();

    app
}

fn entity_at(app: &App, x: i32, y: i32) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(GridPosition::new(x, y))
        .expect("expected indexed block")
}

fn rect_size(collider: &Collider) -> Vec2 {
    let cuboid = collider
        .shape()
        .as_cuboid()
        .expect("collider must be rectangular");

    Vec2::new(cuboid.half_extents.x * 2.0, cuboid.half_extents.y * 2.0)
}

fn damage_sensors(app: &mut App, parent: Entity) -> Vec<RectSnapshot> {
    let world = app.world_mut();

    let mut query =
        world.query_filtered::<
            (
                Entity,
                &Transform,
                &ChildOf,
                &Collider,
            ),
            (
                With<SpikeSensorCollider>,
                With<Sensor>,
            ),
        >();

    let snapshots: Vec<_> = query
        .iter(world)
        .filter(|(_, _, child_of, _)| child_of.0 == parent)
        .map(|(entity, transform, _, collider)| RectSnapshot {
            entity,
            size: rect_size(collider),
            offset: transform.translation.truncate(),
        })
        .collect();

    for snapshot in &snapshots {
        assert_eq!(
            world
                .get::<ColliderOf>(snapshot.entity,)
                .expect(
                    "damage sensor must attach \
                     to parent body",
                )
                .body,
            parent
        );
    }

    snapshots
}

fn solid_colliders(app: &mut App, parent: Entity) -> Vec<RectSnapshot> {
    let world = app.world_mut();

    let mut query = world
        .query_filtered::<(Entity, &Transform, &ChildOf, &Collider), With<SolidColliderChild>>();

    let snapshots: Vec<_> = query
        .iter(world)
        .filter(|(_, _, child_of, _)| child_of.0 == parent)
        .map(|(entity, transform, _, collider)| RectSnapshot {
            entity,
            size: rect_size(collider),
            offset: transform.translation.truncate(),
        })
        .collect();

    for snapshot in &snapshots {
        assert_eq!(
            world
                .get::<ColliderOf>(snapshot.entity,)
                .expect(
                    "solid collider must attach \
                     to parent body",
                )
                .body,
            parent
        );

        assert!(
            world.get::<Sensor>(snapshot.entity,).is_none(),
            "solid collider must not be a sensor"
        );
    }

    snapshots
}

fn vec2_close(actual: Vec2, expected: Vec2) -> bool {
    (actual - expected).length() <= 0.0001
}

fn assert_rectangles(actual: &[RectSnapshot], expected: &[(Vec2, Vec2)]) {
    assert_eq!(actual.len(), expected.len(), "unexpected collider count");

    for (expected_size, expected_offset) in expected {
        assert!(
            actual.iter().any(|snapshot| {
                vec2_close(snapshot.size, *expected_size)
                    && vec2_close(snapshot.offset, *expected_offset)
            },),
            "missing collider size={:?}, \
             offset={:?}; actual={:?}",
            expected_size,
            expected_offset,
            actual
        );
    }
}

#[test]
fn spike_runtime_roles_match_the_unity_prefabs() {
    let app = app_with_spikes();

    for x in [4, 7] {
        let entity = entity_at(&app, x, 0);

        assert!(app.world().get::<DeadlySpike>(entity).is_some());

        assert!(app.world().get::<SolidBlock>(entity).is_none());

        assert_eq!(
            app.world().get::<RigidBody>(entity),
            Some(&RigidBody::Static)
        );

        assert!(app.world().get::<BlockPhysicsBody>(entity,).is_some());
    }

    for x in [10, 13, 16, 19] {
        let entity = entity_at(&app, x, 0);

        assert!(app.world().get::<DeadlySpike>(entity).is_some());

        assert!(app.world().get::<SolidBlock>(entity).is_some());

        assert_eq!(
            app.world().get::<RigidBody>(entity),
            Some(&RigidBody::Static)
        );

        assert!(app.world().get::<BlockPhysicsBody>(entity,).is_some());
    }
}

#[test]
fn simple_spike_sensors_match_unity_geometry() {
    let mut app = app_with_spikes();

    let normal = entity_at(&app, 4, 0);

    assert!(solid_colliders(&mut app, normal,).is_empty());

    assert_rectangles(
        &damage_sensors(&mut app, normal),
        &[(Vec2::new(0.5, 0.5), Vec2::new(0.0, -0.25))],
    );

    let half = entity_at(&app, 7, 0);

    assert!(solid_colliders(&mut app, half,).is_empty());

    assert_rectangles(
        &damage_sensors(&mut app, half),
        &[(Vec2::new(0.5, 0.3), Vec2::new(0.0, -0.35))],
    );
}

#[test]
fn block_spike_colliders_match_unity_geometry() {
    let mut app = app_with_spikes();

    let normal = entity_at(&app, 10, 0);

    assert_rectangles(
        &solid_colliders(&mut app, normal),
        &[(Vec2::new(1.0, 0.5), Vec2::new(0.0, -0.25))],
    );

    assert_rectangles(
        &damage_sensors(&mut app, normal),
        &[(Vec2::new(0.5, 0.2), Vec2::new(0.0, 0.1))],
    );

    let two = entity_at(&app, 13, 0);

    assert_rectangles(
        &solid_colliders(&mut app, two),
        &[(Vec2::new(0.5, 0.5), Vec2::new(0.25, -0.25))],
    );

    assert_rectangles(
        &damage_sensors(&mut app, two),
        &[
            (Vec2::new(0.25, 0.2), Vec2::new(-0.15, -0.25)),
            (Vec2::new(0.2, 0.25), Vec2::new(0.25, 0.15)),
        ],
    );

    let outline_half = entity_at(&app, 16, 0);

    assert_rectangles(
        &solid_colliders(&mut app, outline_half),
        &[(Vec2::new(1.0, 0.5), Vec2::new(0.0, -0.25))],
    );

    assert_rectangles(
        &damage_sensors(&mut app, outline_half),
        &[(Vec2::new(0.8, 0.2), Vec2::new(0.0, 0.1))],
    );

    let outline_two = entity_at(&app, 19, 0);

    assert_rectangles(
        &solid_colliders(&mut app, outline_two),
        &[(Vec2::new(0.5, 0.5), Vec2::new(0.25, -0.25))],
    );

    assert_rectangles(
        &damage_sensors(&mut app, outline_two),
        &[
            (Vec2::new(0.24, 0.4), Vec2::new(-0.12, -0.25)),
            (Vec2::new(0.4, 0.24), Vec2::new(0.25, 0.12)),
        ],
    );
}
