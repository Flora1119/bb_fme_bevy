use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{CardinalDirection, GridPosition, ValidatedMap},
    gameplay::{
        BlockFacing, BlockIdentity, GameplayPhysicsPlugin, GridIndex, MIN_BOUNCE_VELOCITY,
        MapSpawnPlugin, PHYSICS_HZ, PlayRestartPlugin, PlaySession, PlaySessionPlugin,
        PlaySessionState, RestartPlayWorld, SolidColliderChild, SpawnValidatedMap,
        SpikeDeathPlugin, SpikeSensorCollider, spike_collider_profile,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const SPIKE_MAP: &str = include_str!("../assets/maps/phase5a_spikes.json");

const SPIKE_COLUMNS: [(&str, i32); 6] = [
    ("s_normal", 3),
    ("s_half", 7),
    ("s_b_normal", 11),
    ("s_b_two", 15),
    ("s_b_o_half", 19),
    ("s_b_o_two", 23),
];

const DIRECTION_ROWS: [(CardinalDirection, i32); 4] = [
    (CardinalDirection::Up, 2),
    (CardinalDirection::Right, 5),
    (CardinalDirection::Down, 8),
    (CardinalDirection::Left, 11),
];

#[derive(Debug, Clone, Copy)]
struct ColliderSnapshot {
    entity: Entity,
    size: Vec2,
    local_offset: Vec2,
    global_center: Vec2,
}

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(SPIKE_MAP).expect("spike map must deserialize");

    ValidatedMap::from_document(&document, &config).expect("spike map must validate")
}

fn app_with_spike_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
        SpikeDeathPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    // RuntimeBlock 생성
    app.update();

    // Collider 자식 생성 +
    // Transform hierarchy 전파
    app.update();

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / PHYSICS_HZ,
    )));

    app
}

fn entity_at(app: &App, position: GridPosition) -> Entity {
    app.world()
        .resource::<GridIndex>()
        .entity_at(position)
        .unwrap_or_else(|| panic!("expected block at ({}, {})", position.x, position.y))
}

fn player(app: &App) -> Entity {
    entity_at(app, GridPosition::new(1, 4))
}

fn set_player_state(app: &mut App, player: Entity, position: Vec2, velocity: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(velocity),
    ));
}

fn rectangle_size(collider: &Collider) -> Vec2 {
    let cuboid = collider
        .shape()
        .as_cuboid()
        .expect("spike collider must be rectangular");

    Vec2::new(cuboid.half_extents.x * 2.0, cuboid.half_extents.y * 2.0)
}

fn damage_snapshots(app: &mut App, parent: Entity) -> Vec<ColliderSnapshot> {
    let world = app.world_mut();

    let mut query =
        world.query_filtered::<
            (
                Entity,
                &Transform,
                &GlobalTransform,
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
        .filter(|(_, _, _, child_of, _)| child_of.0 == parent)
        .map(
            |(entity, transform, global_transform, _, collider)| ColliderSnapshot {
                entity,
                size: rectangle_size(collider),
                local_offset: transform.translation.truncate(),
                global_center: global_transform.translation().truncate(),
            },
        )
        .collect();

    for snapshot in &snapshots {
        assert_eq!(
            world
                .get::<ColliderOf>(snapshot.entity,)
                .expect(
                    "damage collider must \
                     attach to parent body",
                )
                .body,
            parent
        );
    }

    snapshots
}

fn solid_snapshots(app: &mut App, parent: Entity) -> Vec<ColliderSnapshot> {
    let world = app.world_mut();

    let mut query =
        world.query_filtered::<
            (
                Entity,
                &Transform,
                &GlobalTransform,
                &ChildOf,
                &Collider,
            ),
            With<SolidColliderChild>,
        >();

    let snapshots: Vec<_> = query
        .iter(world)
        .filter(|(_, _, _, child_of, _)| child_of.0 == parent)
        .map(
            |(entity, transform, global_transform, _, collider)| ColliderSnapshot {
                entity,
                size: rectangle_size(collider),
                local_offset: transform.translation.truncate(),
                global_center: global_transform.translation().truncate(),
            },
        )
        .collect();

    for snapshot in &snapshots {
        assert_eq!(
            world
                .get::<ColliderOf>(snapshot.entity,)
                .expect(
                    "solid collider must \
                     attach to parent body",
                )
                .body,
            parent
        );

        assert!(world.get::<Sensor>(snapshot.entity,).is_none());
    }

    snapshots
}

fn all_damage_sensor_entities(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();

    let mut query = world.query_filtered::<Entity, With<SpikeSensorCollider>>();

    query.iter(world).collect()
}

fn all_solid_child_entities(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();

    let mut query = world.query_filtered::<Entity, With<SolidColliderChild>>();

    query.iter(world).collect()
}

fn assert_vec2_close(actual: Vec2, expected: Vec2) {
    const EPSILON: f32 = 0.0001;

    assert!(
        (actual - expected).length() <= EPSILON,
        "expected {expected:?}, \
         found {actual:?}"
    );
}

fn assert_quat_close(actual: Quat, expected: Quat) {
    const EPSILON: f32 = 0.0001;

    let alignment = actual.dot(expected).abs();

    assert!(
        1.0 - alignment <= EPSILON,
        "expected rotation {expected:?}, \
         found {actual:?}"
    );
}

fn snapshot_matches_local_geometry(snapshot: &ColliderSnapshot, size: Vec2, offset: Vec2) -> bool {
    (snapshot.size - size).length() <= 0.0001 && (snapshot.local_offset - offset).length() <= 0.0001
}

fn assert_snapshot_transform(
    snapshot: &ColliderSnapshot,
    parent_transform: &Transform,
    expected_size: Vec2,
    expected_offset: Vec2,
) {
    assert_vec2_close(snapshot.size, expected_size);

    assert_vec2_close(snapshot.local_offset, expected_offset);

    let rotated_offset = (parent_transform.rotation * expected_offset.extend(0.0)).truncate();

    let expected_global_center = parent_transform.translation.truncate() + rotated_offset;

    assert_vec2_close(snapshot.global_center, expected_global_center);
}

fn assert_spike_runtime_geometry(
    app: &mut App,
    id: &str,
    position: GridPosition,
    direction: CardinalDirection,
) {
    let parent = entity_at(app, position);

    let parent_transform = {
        let world = app.world();

        let identity = world
            .get::<BlockIdentity>(parent)
            .expect("spike must have BlockIdentity");

        let facing = world
            .get::<BlockFacing>(parent)
            .expect("spike must have BlockFacing");

        assert_eq!(identity.id.as_str(), id);

        assert_eq!(facing.0, direction);

        world
            .get::<Transform>(parent)
            .expect("spike must have Transform")
            .clone()
    };

    let expected_rotation =
        Quat::from_rotation_z((direction.unity_angle_degrees() as f32).to_radians());

    assert_quat_close(parent_transform.rotation, expected_rotation);

    let profile = spike_collider_profile(id).expect(
        "known spike must \
                 have a collider profile",
    );

    let solids = solid_snapshots(app, parent);

    match profile.solid() {
        Some(expected) => {
            assert_eq!(
                solids.len(),
                1,
                "{id} must have one \
                 solid collider"
            );

            assert_snapshot_transform(
                &solids[0],
                &parent_transform,
                expected.size(),
                expected.offset(),
            );
        }

        None => {
            assert!(
                solids.is_empty(),
                "{id} must not have \
                 a solid collider"
            );
        }
    }

    let damage = damage_snapshots(app, parent);

    assert_eq!(
        damage.len(),
        profile.damage_sensors().len(),
        "{id} has wrong damage \
         sensor count"
    );

    for expected in profile.damage_sensors() {
        let snapshot = damage
            .iter()
            .find(|snapshot| {
                snapshot_matches_local_geometry(snapshot, expected.size(), expected.offset())
            })
            .unwrap_or_else(|| {
                panic!(
                    "{id} is missing damage \
                     sensor size={:?}, \
                     offset={:?}",
                    expected.size(),
                    expected.offset(),
                )
            });

        assert_snapshot_transform(
            snapshot,
            &parent_transform,
            expected.size(),
            expected.offset(),
        );
    }
}

#[test]
fn phase5a_spike_map_contains_six_profiles_in_four_directions() {
    let map = load_validated_map();

    assert_eq!(map.blocks.len(), 27);

    for (id, _) in SPIKE_COLUMNS {
        assert_eq!(
            map.blocks
                .iter()
                .filter(|block| { block.id.as_str() == id },)
                .count(),
            4,
            "{id} must appear four times"
        );

        for (direction, _) in DIRECTION_ROWS {
            assert!(
                map.blocks
                    .iter()
                    .any(|block| { block.id.as_str() == id && block.direction == direction },),
                "{id} is missing \
                 direction {direction:?}"
            );
        }
    }
}

#[test]
fn all_spike_colliders_follow_all_four_map_rotations() {
    let mut app = app_with_spike_map();

    for (direction, y) in DIRECTION_ROWS {
        for (id, x) in SPIKE_COLUMNS {
            assert_spike_runtime_geometry(&mut app, id, GridPosition::new(x, y), direction);
        }
    }

    assert_eq!(
        all_damage_sensor_entities(&mut app,).len(),
        32,
        "six spike profiles across \
         four rotations must create \
         32 damage rectangles"
    );

    assert_eq!(
        all_solid_child_entities(&mut app,).len(),
        16,
        "four block-spike profiles \
         across four rotations must \
         create 16 solid children"
    );
}

#[test]
fn every_rotated_damage_rectangle_kills_the_player() {
    let mut app = app_with_spike_map();

    let player = player(&app);

    // 사망 Rectangle의 순수 충돌 판정만
    // 보기 위해 중력을 제거합니다.
    app.world_mut().resource_mut::<Gravity>().0 = Vec2::ZERO;

    let mut targets = Vec::new();

    for (_, y) in DIRECTION_ROWS {
        for (_, x) in SPIKE_COLUMNS {
            let parent = entity_at(&app, GridPosition::new(x, y));

            for sensor in damage_snapshots(&mut app, parent) {
                targets.push(sensor.global_center);
            }
        }
    }

    assert_eq!(targets.len(), 32);

    for (index, target) in targets.into_iter().enumerate() {
        // 이전 Sensor와의 접촉을 완전히
        // 끊은 뒤 다음 케이스를 검사합니다.
        set_player_state(&mut app, player, Vec2::new(1.0, 13.0), Vec2::ZERO);

        app.update();
        app.update();

        app.world_mut().resource_mut::<PlaySession>().reset();

        set_player_state(&mut app, player, target, Vec2::ZERO);

        for _ in 0..4 {
            app.update();

            if app.world().resource::<PlaySession>().state() == PlaySessionState::Dead {
                break;
            }
        }

        assert_eq!(
            app.world().resource::<PlaySession>().state(),
            PlaySessionState::Dead,
            "damage rectangle {index} \
             did not kill the player \
             at {target:?}"
        );
    }
}

#[test]
fn composite_damage_sensor_is_not_a_solid_surface() {
    let mut app = app_with_spike_map();

    let player = player(&app);

    // Up 방향 s_b_normal:
    //
    // Solid:
    // y = 1.5 .. 2.0
    //
    // Damage:
    // y = 2.0 .. 2.2
    //
    // 공 중심을 y=2.35에 놓으면
    // Damage Trigger에는 겹치지만
    // 실제 Solid에는 닿지 않습니다.
    set_player_state(
        &mut app,
        player,
        Vec2::new(11.0, 2.35),
        Vec2::new(0.0, -1.0),
    );

    app.update();

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Dead,
        "damage rectangle must kill \
         the player"
    );

    let velocity = app
        .world()
        .get::<LinearVelocity>(player)
        .expect("player must have velocity")
        .0;

    assert!(
        velocity.y < MIN_BOUNCE_VELOCITY * 0.5,
        "damage sensor was incorrectly \
         treated as a solid bounce \
         surface: velocity={velocity:?}"
    );
}

#[test]
fn composite_spikes_bounce_from_their_safe_solid_faces() {
    let cases = [
        ("s_b_normal", GridPosition::new(11, 8), Vec2::new(11.0, 9.5)),
        ("s_b_two", GridPosition::new(15, 8), Vec2::new(14.72, 9.5)),
        ("s_b_o_half", GridPosition::new(19, 8), Vec2::new(19.0, 9.5)),
        ("s_b_o_two", GridPosition::new(23, 8), Vec2::new(22.72, 9.5)),
    ];

    for (id, position, start_position) in cases {
        let mut app = app_with_spike_map();

        let player = player(&app);

        // 실제 테스트 대상이
        // Down 방향인지 먼저 보장합니다.
        let spike = entity_at(&app, position);

        assert_eq!(
            app.world()
                .get::<BlockFacing>(spike)
                .expect("spike needs facing",)
                .0,
            CardinalDirection::Down
        );

        set_player_state(&mut app, player, start_position, Vec2::ZERO);

        let mut bounced = false;

        for _ in 0..40 {
            app.update();

            let velocity = app
                .world()
                .get::<LinearVelocity>(player)
                .expect("player must have velocity")
                .0;

            if velocity.y >= MIN_BOUNCE_VELOCITY - 0.1 {
                bounced = true;

                break;
            }
        }

        assert!(
            bounced,
            "{id} did not bounce \
             from its safe solid face"
        );

        assert_eq!(
            app.world().resource::<PlaySession>().state(),
            PlaySessionState::Playing,
            "{id} solid-side bounce \
             incorrectly touched a \
             damage rectangle"
        );
    }
}

#[test]
fn restart_recreates_every_spike_collider_and_resets_death() {
    let mut app = app_with_spike_map();

    let old_parent = entity_at(&app, GridPosition::new(23, 5));

    let old_damage = damage_snapshots(&mut app, old_parent)[0].entity;

    let old_solid = solid_snapshots(&mut app, old_parent)[0].entity;

    assert_eq!(all_damage_sensor_entities(&mut app,).len(), 32);

    assert_eq!(all_solid_child_entities(&mut app,).len(), 16);

    app.world_mut().resource_mut::<PlaySession>().mark_dead();

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Dead
    );

    app.world_mut().write_message(RestartPlayWorld);

    // Restart + 새 RuntimeBlock
    app.update();

    // 새 Collider 생성 +
    // hierarchy 전파
    app.update();

    assert!(
        !app.world().entities().contains(old_parent),
        "old spike parent survived restart"
    );

    assert!(
        !app.world().entities().contains(old_damage),
        "old damage collider \
         survived restart"
    );

    assert!(
        !app.world().entities().contains(old_solid),
        "old solid collider \
         survived restart"
    );

    assert_eq!(
        app.world().resource::<PlaySession>().state(),
        PlaySessionState::Playing,
        "restart must reset death state"
    );

    assert_eq!(
        all_damage_sensor_entities(&mut app,).len(),
        32,
        "restart must recreate all \
         32 damage rectangles"
    );

    assert_eq!(
        all_solid_child_entities(&mut app,).len(),
        16,
        "restart must recreate all \
         16 spike solid colliders"
    );

    let new_parent = entity_at(&app, GridPosition::new(23, 5));

    assert_ne!(new_parent, old_parent);

    assert_spike_runtime_geometry(
        &mut app,
        "s_b_o_two",
        GridPosition::new(23, 5),
        CardinalDirection::Right,
    );
}
