use super::{
    BLOCK_WORLD_SIZE, MapSpawnSet, PendingPlayInteractions, PlayInteraction,
    PlayInteractionCollectSet, PlayInteractionSet, PlayWorld, PlayerBall,
};
use avian2d::prelude::*;
use bevy::prelude::*;

pub const MAP_BORDER_OFFSET: f32 = 2.0 * BLOCK_WORLD_SIZE;

pub const MAP_BORDER_THICKNESS: f32 = 2.0 * BLOCK_WORLD_SIZE;

const MAP_BORDER_COLOR: Color = Color::srgb(1.0, 0.25, 0.8);

pub struct MapBoundaryPlugin;

impl Plugin for MapBoundaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_map_boundaries.after(MapSpawnSet))
            .add_systems(
                PhysicsSchedule,
                collect_started_boundary_interactions
                    .in_set(PlayInteractionSet::Collect)
                    .in_set(PlayInteractionCollectSet::BoundaryDeath),
            );
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapBoundary {
    Left,
    Right,
    Top,
    Bottom,
}

fn spawn_map_boundaries(
    mut commands: Commands,
    play_worlds: Query<(Entity, &PlayWorld), Added<PlayWorld>>,
) {
    for (root, play_world) in &play_worlds {
        let size = play_world.definition().settings.size;

        let width = size.width as f32 * BLOCK_WORLD_SIZE;

        let height = size.height as f32 * BLOCK_WORLD_SIZE;

        spawn_boundary(
            &mut commands,
            root,
            "Map Boundary: Left",
            MapBoundary::Left,
            Vec2::new(-MAP_BORDER_OFFSET, height * 0.5),
            Vec2::new(MAP_BORDER_THICKNESS, MAP_BORDER_THICKNESS + height),
        );

        spawn_boundary(
            &mut commands,
            root,
            "Map Boundary: Right",
            MapBoundary::Right,
            Vec2::new(width + MAP_BORDER_OFFSET, height * 0.5),
            Vec2::new(MAP_BORDER_THICKNESS, MAP_BORDER_THICKNESS + height),
        );

        spawn_boundary(
            &mut commands,
            root,
            "Map Boundary: Top",
            MapBoundary::Top,
            Vec2::new(width * 0.5, height + MAP_BORDER_OFFSET),
            Vec2::new(MAP_BORDER_THICKNESS * 3.0 + width, MAP_BORDER_THICKNESS),
        );

        spawn_boundary(
            &mut commands,
            root,
            "Map Boundary: Bottom",
            MapBoundary::Bottom,
            Vec2::new(width * 0.5, -MAP_BORDER_OFFSET),
            Vec2::new(MAP_BORDER_THICKNESS * 3.0 + width, MAP_BORDER_THICKNESS),
        );
    }
}

fn spawn_boundary(
    commands: &mut Commands,
    root: Entity,
    name: &'static str,
    boundary: MapBoundary,
    position: Vec2,
    size: Vec2,
) {
    commands.spawn((
        Name::new(name),
        boundary,
        RigidBody::Static,
        Sensor,
        CollisionEventsEnabled,
        Collider::rectangle(size.x, size.y),
        Transform::from_translation(position.extend(0.0)),
        DebugRender::default().with_collider_color(MAP_BORDER_COLOR),
        ChildOf(root),
    ));
}

fn collect_started_boundary_interactions(
    mut collision_starts: MessageReader<CollisionStart>,
    players: Query<(), With<PlayerBall>>,
    boundaries: Query<(), With<MapBoundary>>,
    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let boundary = if players.contains(event.collider1) && boundaries.contains(event.collider2)
        {
            event.collider2
        } else if players.contains(event.collider2) && boundaries.contains(event.collider1) {
            event.collider1
        } else {
            continue;
        };

        pending.push(PlayInteraction::death(boundary));
    }
}
