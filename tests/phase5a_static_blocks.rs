use avian2d::prelude::*;
use bb_fme_bevy::{
    block::BlockAssetConfig,
    domain::{CardinalDirection, GridPosition, ValidatedMap},
    gameplay::{
        BLOCK_WORLD_SIZE, BlockFacing, BlockIdentity, GameplayPhysicsPlugin, GridIndex,
        MIN_BOUNCE_VELOCITY, MapSpawnPlugin, PHYSICS_HZ, PlayRestartPlugin, PlaySessionPlugin,
        RestartPlayWorld, SolidColliderChild, SpawnValidatedMap,
    },
    map::MapDocument,
};
use bevy::{
    asset::Assets, gizmos::GizmoAsset, input::InputPlugin, prelude::*, time::TimeUpdateStrategy,
    transform::TransformPlugin,
};
use std::time::Duration;

const BLOCK_CONFIG: &str = include_str!("../assets/config/block_assets_config.json");

const STATIC_BLOCK_MAP: &str = include_str!("../assets/maps/phase5a_static_blocks.json");

#[derive(Debug, Clone, Copy)]
struct PartialColliderSnapshot {
    entity: Entity,
    local_offset: Vec2,
    global_center: Vec2,
    size: Vec2,
}

fn load_validated_map() -> ValidatedMap {
    let config: BlockAssetConfig =
        serde_json::from_str(BLOCK_CONFIG).expect("block config must deserialize");

    let document: MapDocument =
        serde_json::from_str(STATIC_BLOCK_MAP).expect("static block map must deserialize");

    ValidatedMap::from_document(&document, &config).expect("static block map must validate")
}

fn app_with_static_block_map() -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        InputPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
        PlayRestartPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    app.world_mut()
        .write_message(SpawnValidatedMap(load_validated_map()));

    // 첫 Update:
    // PlayWorld와 RuntimeBlock 생성.
    app.update();

    // 두 번째 Update:
    // 부분 블록의 자식 Collider 생성 및
    // Transform 계층 전파를 확실하게 완료.
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

fn set_player_state(app: &mut App, player: Entity, position: Vec2, velocity: Vec2) {
    app.world_mut().entity_mut(player).insert((
        Position(position),
        Transform::from_translation(position.extend(0.0)),
        LinearVelocity(velocity),
    ));
}

fn assert_vec2_close(actual: Vec2, expected: Vec2) {
    const EPSILON: f32 = 0.0001;

    assert!(
        (actual - expected).length() <= EPSILON,
        "expected {expected:?}, found {actual:?}"
    );
}

fn assert_quat_close(actual: Quat, expected: Quat) {
    const EPSILON: f32 = 0.0001;

    // q와 -q는 동일한 회전을 나타내므로
    // dot의 절댓값으로 비교합니다.
    let alignment = actual.dot(expected).abs();

    assert!(
        1.0 - alignment <= EPSILON,
        "expected rotation {expected:?}, \
         found {actual:?}"
    );
}

fn partial_collider_snapshot(app: &mut App, parent: Entity) -> PartialColliderSnapshot {
    let world = app.world_mut();

    let mut colliders = world.query_filtered::<
        (
            Entity,
            &Transform,
            &GlobalTransform,
            &ChildOf,
            &Collider,
        ),
        With<SolidColliderChild>,
    >();

    let matches: Vec<PartialColliderSnapshot> = colliders
        .iter(world)
        .filter(|(_, _, _, child_of, _)| child_of.0 == parent)
        .map(|(entity, transform, global_transform, _, collider)| {
            let cuboid = collider.shape().as_cuboid().expect(
                "partial solid collider \
                             must be rectangular",
            );

            PartialColliderSnapshot {
                entity,
                local_offset: transform.translation.truncate(),
                global_center: global_transform.translation().truncate(),
                size: Vec2::new(cuboid.half_extents.x * 2.0, cuboid.half_extents.y * 2.0),
            }
        })
        .collect();

    assert_eq!(
        matches.len(),
        1,
        "partial solid must have exactly one \
         SolidColliderChild"
    );

    let snapshot = matches[0];

    assert_eq!(
        world
            .get::<ColliderOf>(snapshot.entity)
            .expect(
                "partial collider must attach \
                 to its parent rigid body",
            )
            .body,
        parent
    );

    snapshot
}

fn assert_partial_runtime_geometry(
    app: &mut App,
    position: GridPosition,
    expected_id: &str,
    expected_direction: CardinalDirection,
    expected_size: Vec2,
    expected_local_offset: Vec2,
) {
    let parent = entity_at(app, position);

    let (identity, facing, parent_transform) = {
        let world = app.world();

        (
            world
                .get::<BlockIdentity>(parent)
                .expect("block must have BlockIdentity")
                .clone(),
            *world
                .get::<BlockFacing>(parent)
                .expect("block must have BlockFacing"),
            world
                .get::<Transform>(parent)
                .expect("block must have Transform")
                .clone(),
        )
    };

    assert_eq!(identity.id.as_str(), expected_id);

    assert_eq!(facing.0, expected_direction);

    let expected_rotation =
        Quat::from_rotation_z((expected_direction.unity_angle_degrees() as f32).to_radians());

    assert_quat_close(parent_transform.rotation, expected_rotation);

    let snapshot = partial_collider_snapshot(app, parent);

    assert_vec2_close(snapshot.size, expected_size);

    assert_vec2_close(snapshot.local_offset, expected_local_offset);

    // 자식 Collider의 로컬 offset이
    // 부모 회전에 의해 실제 월드 위치로
    // 회전되었는지 확인합니다.
    let rotated_offset = (parent_transform.rotation * expected_local_offset.extend(0.0)).truncate();

    let expected_global_center = parent_transform.translation.truncate() + rotated_offset;

    assert_vec2_close(snapshot.global_center, expected_global_center);
}

fn partial_collider_entities(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();

    let mut colliders = world.query_filtered::<Entity, With<SolidColliderChild>>();

    colliders.iter(world).collect()
}

#[test]
fn phase5a_map_contains_all_static_block_profiles() {
    let map = load_validated_map();

    assert_eq!(map.blocks.len(), 12);

    for expected_id in ["b_normal", "b_o", "b_o_half", "b_o_quarter"] {
        assert!(
            map.blocks
                .iter()
                .any(|block| { block.id.as_str() == expected_id },),
            "phase5a map is missing {expected_id}"
        );
    }
}

#[test]
fn partial_static_block_colliders_follow_all_four_map_rotations() {
    let mut app = app_with_static_block_map();

    let half_size = Vec2::new(BLOCK_WORLD_SIZE, BLOCK_WORLD_SIZE * 0.5);

    let half_offset = Vec2::new(0.0, -BLOCK_WORLD_SIZE * 0.25);

    for (position, direction) in [
        (GridPosition::new(4, 0), CardinalDirection::Up),
        (GridPosition::new(7, 2), CardinalDirection::Right),
        (GridPosition::new(10, 2), CardinalDirection::Down),
        (GridPosition::new(13, 2), CardinalDirection::Left),
    ] {
        assert_partial_runtime_geometry(
            &mut app,
            position,
            "b_o_half",
            direction,
            half_size,
            half_offset,
        );
    }

    let quarter_size = Vec2::new(BLOCK_WORLD_SIZE * 0.5, BLOCK_WORLD_SIZE * 0.5);

    let quarter_offset = Vec2::new(BLOCK_WORLD_SIZE * 0.25, -BLOCK_WORLD_SIZE * 0.25);

    for (position, direction) in [
        (GridPosition::new(4, 6), CardinalDirection::Up),
        (GridPosition::new(7, 6), CardinalDirection::Right),
        (GridPosition::new(10, 6), CardinalDirection::Down),
        (GridPosition::new(13, 6), CardinalDirection::Left),
    ] {
        assert_partial_runtime_geometry(
            &mut app,
            position,
            "b_o_quarter",
            direction,
            quarter_size,
            quarter_offset,
        );
    }
}

#[test]
fn up_facing_half_and_quarter_blocks_really_bounce_the_player() {
    let cases = [
        // b_o_half:
        // collider 중심 x = 부모 중심 x
        (Vec2::new(4.0, 2.0), "b_o_half"),
        // b_o_quarter:
        // 로컬 collider 중심이 x + 0.25이므로
        // 정확히 그 중심 위로 공을 떨어뜨립니다.
        (Vec2::new(4.25, 8.0), "b_o_quarter"),
    ];

    for (start_position, block_name) in cases {
        let mut app = app_with_static_block_map();

        let player = entity_at(&app, GridPosition::new(4, 4));

        set_player_state(&mut app, player, start_position, Vec2::ZERO);

        let mut strongest_upward_speed = f32::NEG_INFINITY;

        for _ in 0..120 {
            app.update();

            let velocity = app
                .world()
                .get::<LinearVelocity>(player)
                .expect("player must have LinearVelocity")
                .0;

            strongest_upward_speed = strongest_upward_speed.max(velocity.y);
        }

        assert!(
            strongest_upward_speed >= MIN_BOUNCE_VELOCITY - 0.1,
            "{block_name} did not produce \
             the expected bounce: \
             strongest upward speed was \
             {strongest_upward_speed}"
        );
    }
}

#[test]
fn restart_recreates_all_partial_block_colliders() {
    let mut app = app_with_static_block_map();

    let old_half_parent = entity_at(&app, GridPosition::new(7, 2));

    let old_quarter_parent = entity_at(&app, GridPosition::new(7, 6));

    let old_half_child = partial_collider_snapshot(&mut app, old_half_parent).entity;

    let old_quarter_child = partial_collider_snapshot(&mut app, old_quarter_parent).entity;

    let old_partial_colliders = partial_collider_entities(&mut app);

    assert_eq!(
        old_partial_colliders.len(),
        8,
        "expected four half and four \
         quarter collider children"
    );

    app.world_mut().write_message(RestartPlayWorld);

    // Restart 요청 처리 + 새 맵 생성
    app.update();

    // 새 블록 Collider 부착 및
    // Transform 계층 전파 완료
    app.update();

    assert!(
        !app.world().entities().contains(old_half_parent),
        "old half block parent survived restart"
    );

    assert!(
        !app.world().entities().contains(old_half_child),
        "old half collider child survived restart"
    );

    assert!(
        !app.world().entities().contains(old_quarter_parent),
        "old quarter block parent survived restart"
    );

    assert!(
        !app.world().entities().contains(old_quarter_child),
        "old quarter collider child survived restart"
    );

    let new_half_parent = entity_at(&app, GridPosition::new(7, 2));

    let new_quarter_parent = entity_at(&app, GridPosition::new(7, 6));

    assert_ne!(new_half_parent, old_half_parent);

    assert_ne!(new_quarter_parent, old_quarter_parent);

    let new_partial_colliders = partial_collider_entities(&mut app);

    assert_eq!(
        new_partial_colliders.len(),
        8,
        "restart must recreate all eight \
         partial collider children"
    );

    assert_partial_runtime_geometry(
        &mut app,
        GridPosition::new(7, 2),
        "b_o_half",
        CardinalDirection::Right,
        Vec2::new(BLOCK_WORLD_SIZE, BLOCK_WORLD_SIZE * 0.5),
        Vec2::new(0.0, -BLOCK_WORLD_SIZE * 0.25),
    );

    assert_partial_runtime_geometry(
        &mut app,
        GridPosition::new(7, 6),
        "b_o_quarter",
        CardinalDirection::Right,
        Vec2::new(BLOCK_WORLD_SIZE * 0.5, BLOCK_WORLD_SIZE * 0.5),
        Vec2::new(BLOCK_WORLD_SIZE * 0.25, -BLOCK_WORLD_SIZE * 0.25),
    );
}
