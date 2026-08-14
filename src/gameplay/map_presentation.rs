use super::{
    BLOCK_WORLD_SIZE, BlockVisualSet, CollectibleStar, MapSpawnSet, PlayWorld, PlayerBall,
    SolidBlock,
};
use bevy::{camera::ScalingMode, prelude::*};

pub const MIN_VIEW_WIDTH: f32 = 25.0;
pub const MIN_VIEW_HEIGHT: f32 = 15.0;

pub const PLAYER_VISUAL_SIZE: Vec2 = Vec2::splat(0.72 * BLOCK_WORLD_SIZE);
pub const STAR_VISUAL_SIZE: Vec2 = Vec2::splat(0.60 * BLOCK_WORLD_SIZE);
pub const SOLID_VISUAL_SIZE: Vec2 = Vec2::splat(0.96 * BLOCK_WORLD_SIZE);

pub const PLAYER_COLOR: Color = Color::srgb(0.15, 0.80, 1.00);
pub const STAR_COLOR: Color = Color::srgb(1.00, 0.82, 0.12);
pub const SOLID_COLOR: Color = Color::srgb(0.30, 0.36, 0.46);

pub struct MapPresentationPlugin;

impl Plugin for MapPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.08)))
            .add_systems(Startup, spawn_map_camera)
            .add_systems(
                Update,
                (
                    frame_camera_for_loaded_map,
                    add_player_visuals,
                    add_star_visuals,
                    add_solid_visuals,
                )
                    .in_set(MapPresentationSet)
                    .after(MapSpawnSet)
                    .after(BlockVisualSet),
            );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapPresentationSet;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCamera;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceholderVisual;

fn spawn_map_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("MapCamera"),
        MapCamera,
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: MIN_VIEW_WIDTH,
                min_height: MIN_VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn frame_camera_for_loaded_map(
    play_worlds: Query<&PlayWorld, Added<PlayWorld>>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<MapCamera>>,
) {
    let Some(play_world) = play_worlds.iter().last() else {
        return;
    };

    let size = play_world.definition().settings.size;
    let view_width = (size.width as f32 * BLOCK_WORLD_SIZE).max(MIN_VIEW_WIDTH);
    let view_height = (size.height as f32 * BLOCK_WORLD_SIZE).max(MIN_VIEW_HEIGHT);
    let center_x = (size.width - 1) as f32 * BLOCK_WORLD_SIZE * 0.5;
    let center_y = (size.height - 1) as f32 * BLOCK_WORLD_SIZE * 0.5;

    for (mut transform, mut projection) in &mut cameras {
        transform.translation.x = center_x;
        transform.translation.y = center_y;

        let Projection::Orthographic(orthographic) = projection.as_mut() else {
            continue;
        };

        orthographic.scaling_mode = ScalingMode::AutoMin {
            min_width: view_width,
            min_height: view_height,
        };
    }
}

fn add_player_visuals(
    mut commands: Commands,
    players: Query<Entity, (Added<PlayerBall>, Without<Sprite>)>,
) {
    for entity in &players {
        commands.entity(entity).insert((
            PlaceholderVisual,
            Sprite::from_color(PLAYER_COLOR, PLAYER_VISUAL_SIZE),
        ));
    }
}

fn add_star_visuals(
    mut commands: Commands,
    stars: Query<Entity, (Added<CollectibleStar>, Without<Sprite>)>,
) {
    for entity in &stars {
        commands.entity(entity).insert((
            PlaceholderVisual,
            Sprite::from_color(STAR_COLOR, STAR_VISUAL_SIZE),
        ));
    }
}

fn add_solid_visuals(
    mut commands: Commands,
    solids: Query<Entity, (Added<SolidBlock>, Without<Sprite>)>,
) {
    for entity in &solids {
        commands.entity(entity).insert((
            PlaceholderVisual,
            Sprite::from_color(SOLID_COLOR, SOLID_VISUAL_SIZE),
        ));
    }
}
