use super::{
    BLOCK_WORLD_SIZE, CollectedStar, CollectibleStar, MapSpawnSet, PendingPlayInteractions,
    PlayInteraction, PlayInteractionSet, PlayerBall,
};
use avian2d::prelude::*;
use bevy::prelude::*;

pub const STAR_SENSOR_RADIUS: f32 = 0.5 * BLOCK_WORLD_SIZE;

const STAR_SENSOR_COLOR: Color = Color::srgb(1.0, 0.85, 0.1);

pub struct StarCollectionPlugin;

impl Plugin for StarCollectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_star_sensors.after(MapSpawnSet))
            .add_systems(
                PhysicsSchedule,
                collect_started_star_interactions.in_set(PlayInteractionSet::Collect),
            );
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarSensorCollider;

fn attach_star_sensors(
    mut commands: Commands,
    stars: Query<
        Entity,
        (
            With<CollectibleStar>,
            Without<CollectedStar>,
            Without<StarSensorCollider>,
        ),
    >,
) {
    for entity in &stars {
        commands.entity(entity).insert((
            StarSensorCollider,
            Sensor,
            CollisionEventsEnabled,
            Collider::circle(STAR_SENSOR_RADIUS),
            DebugRender::default().with_collider_color(STAR_SENSOR_COLOR),
        ));
    }
}

fn collect_started_star_interactions(
    mut collision_starts: MessageReader<CollisionStart>,
    players: Query<(), With<PlayerBall>>,
    stars: Query<(), (With<CollectibleStar>, Without<CollectedStar>)>,
    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let star = if players.contains(event.collider1) && stars.contains(event.collider2) {
            event.collider2
        } else if players.contains(event.collider2) && stars.contains(event.collider1) {
            event.collider1
        } else {
            continue;
        };

        pending.push(PlayInteraction::collection(star));
    }
}
