use avian2d::prelude::*;
use bb_fme_bevy::{
    block::{BlockCategory, BlockId},
    gameplay::{
        BLOCK_WORLD_SIZE, BlockIdentity, BlockPhysicsBody, GameplayPhysicsPlugin, MapSpawnPlugin,
        PlaySessionPlugin, SolidBlock, SolidColliderChild,
    },
};
use bevy::{asset::Assets, gizmos::GizmoAsset, prelude::*, transform::TransformPlugin};

struct StaticBlockEntities {
    normal: Entity,
    outline: Entity,
    half: Entity,
    quarter: Entity,
    function_solid: Entity,
    anonymous_solid: Entity,
}

fn spawn_identified_solid(
    app: &mut App,
    block_id: &str,
    category: BlockCategory,
    x: f32,
) -> Entity {
    app.world_mut()
        .spawn((
            Name::new(format!("Test solid: {block_id}")),
            SolidBlock,
            BlockIdentity {
                id: BlockId::from(block_id),
                category,
            },
            Transform::from_xyz(x, 0.0, 0.0),
        ))
        .id()
}

fn app_with_static_blocks() -> (App, StaticBlockEntities) {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        MapSpawnPlugin,
        GameplayPhysicsPlugin,
        PlaySessionPlugin,
    ));

    app.init_resource::<Assets<GizmoAsset>>();

    app.finish();
    app.cleanup();

    let normal = spawn_identified_solid(&mut app, "b_normal", BlockCategory::Block, 0.0);

    let outline = spawn_identified_solid(&mut app, "b_o", BlockCategory::Block, 2.0);

    let half = spawn_identified_solid(&mut app, "b_o_half", BlockCategory::Block, 4.0);

    let quarter = spawn_identified_solid(&mut app, "b_o_quarter", BlockCategory::Block, 6.0);

    let function_solid = spawn_identified_solid(&mut app, "fb_jump", BlockCategory::Funcblock, 8.0);

    let anonymous_solid = app
        .world_mut()
        .spawn((
            Name::new("Anonymous test solid"),
            SolidBlock,
            Transform::from_xyz(10.0, 0.0, 0.0),
        ))
        .id();

    app.update();
    app.update();

    (
        app,
        StaticBlockEntities {
            normal,
            outline,
            half,
            quarter,
            function_solid,
            anonymous_solid,
        },
    )
}

fn assert_full_tile_parent_collider(app: &App, entity: Entity) {
    assert_eq!(
        app.world().get::<RigidBody>(entity),
        Some(&RigidBody::Static)
    );

    assert!(app.world().get::<BlockPhysicsBody>(entity).is_some());

    let collider = app
        .world()
        .get::<Collider>(entity)
        .expect("full-tile solid must have a parent collider");

    let cuboid = collider
        .shape()
        .as_cuboid()
        .expect("solid collider must be rectangular");

    assert_eq!(cuboid.half_extents.x, BLOCK_WORLD_SIZE * 0.5);

    assert_eq!(cuboid.half_extents.y, BLOCK_WORLD_SIZE * 0.5);

    assert!(app.world().get::<DebugRender>(entity).is_some());
}

fn assert_offset_child_collider(
    app: &mut App,
    parent: Entity,
    expected_size: Vec2,
    expected_offset: Vec2,
) {
    assert_eq!(
        app.world().get::<RigidBody>(parent),
        Some(&RigidBody::Static)
    );

    assert!(app.world().get::<BlockPhysicsBody>(parent).is_some());

    assert!(
        app.world().get::<Collider>(parent).is_none(),
        "offset solid must keep its collider on a child entity"
    );

    let world = app.world_mut();

    let mut colliders = world
        .query_filtered::<(Entity, &Transform, &ChildOf, &Collider), With<SolidColliderChild>>();

    let matching: Vec<(Entity, Vec2, Vec2)> = colliders
        .iter(world)
        .filter(|(_, _, child_of, _)| child_of.0 == parent)
        .map(|(entity, transform, _, collider)| {
            let cuboid = collider
                .shape()
                .as_cuboid()
                .expect("offset solid collider must be rectangular");

            (
                entity,
                transform.translation.truncate(),
                Vec2::new(cuboid.half_extents.x * 2.0, cuboid.half_extents.y * 2.0),
            )
        })
        .collect();

    assert_eq!(
        matching.len(),
        1,
        "offset solid must have exactly one collider child"
    );

    let (collider_entity, offset, size) = matching[0];

    assert_eq!(offset, expected_offset);
    assert_eq!(size, expected_size);

    assert_eq!(
        world
            .get::<ColliderOf>(collider_entity)
            .expect("child collider must attach to the parent body")
            .body,
        parent
    );

    assert!(world.get::<DebugRender>(collider_entity).is_some());
}

#[test]
fn full_static_blocks_keep_the_unity_full_tile_collider() {
    let (app, entities) = app_with_static_blocks();

    assert_full_tile_parent_collider(&app, entities.normal);

    assert_full_tile_parent_collider(&app, entities.outline);
}

#[test]
fn partial_static_blocks_use_the_unity_offset_colliders() {
    let (mut app, entities) = app_with_static_blocks();

    assert_offset_child_collider(
        &mut app,
        entities.half,
        Vec2::new(BLOCK_WORLD_SIZE, BLOCK_WORLD_SIZE * 0.5),
        Vec2::new(0.0, -BLOCK_WORLD_SIZE * 0.25),
    );

    assert_offset_child_collider(
        &mut app,
        entities.quarter,
        Vec2::new(BLOCK_WORLD_SIZE * 0.5, BLOCK_WORLD_SIZE * 0.5),
        Vec2::new(BLOCK_WORLD_SIZE * 0.25, -BLOCK_WORLD_SIZE * 0.25),
    );
}

#[test]
fn other_solid_blocks_keep_the_full_tile_fallback() {
    let (app, entities) = app_with_static_blocks();

    assert_full_tile_parent_collider(&app, entities.function_solid);

    assert_full_tile_parent_collider(&app, entities.anonymous_solid);
}
