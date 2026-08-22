use super::{
    BLOCK_WORLD_SIZE, BlockOptions, CollectedStar, CollectibleStar, MapSpawnSet,
    PendingPlayInteractions, PlayInteraction, PlayInteractionCollectSet, PlayInteractionSet,
    PlayerBall, TransparentStar,
};
use avian2d::prelude::*;
use bevy::prelude::*;

pub const STAR_SENSOR_RADIUS: f32 = 0.4 * BLOCK_WORLD_SIZE;

pub const DEFAULT_STAR_SCALE: f32 = 1.0;

const STAR_SCALE_OPTION_NAME: &str = "Scale";

const STAR_SENSOR_COLOR: Color = Color::srgb(1.0, 0.85, 0.1);

pub struct StarCollectionPlugin;

impl Plugin for StarCollectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_star_sensors.after(MapSpawnSet))
            .add_systems(
                PhysicsSchedule,
                collect_started_star_interactions
                    .in_set(PlayInteractionSet::Collect)
                    .in_set(PlayInteractionCollectSet::Collection),
            );
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarSensorCollider;

fn star_scale_from_options(options: Option<&BlockOptions>) -> f32 {
    options
        .and_then(|options| {
            options
                .0
                .iter()
                .find(|option| option.name == STAR_SCALE_OPTION_NAME)
        })
        .map(|option| option.value)
        .unwrap_or(DEFAULT_STAR_SCALE)
}

fn attach_star_sensors(
    mut commands: Commands,
    mut stars: Query<
        (
            Entity,
            Option<&BlockOptions>,
            Option<&TransparentStar>,
            &mut Transform,
        ),
        (
            With<CollectibleStar>,
            Without<CollectedStar>,
            Without<StarSensorCollider>,
        ),
    >,
) {
    for (entity, options, transparent_star, mut transform) in &mut stars {
        let scale = star_scale_from_options(options);

        // Unity의 transform.localScale과 동일하게
        // Sprite와 Collider를 함께 확대/축소합니다.
        transform.scale = Vec3::new(scale, scale, 1.0);

        let mut entity_commands = commands.entity(entity);

        entity_commands.insert((
            StarSensorCollider,
            Sensor,
            CollisionEventsEnabled,
            Collider::circle(STAR_SENSOR_RADIUS),
            DebugRender::default().with_collider_color(STAR_SENSOR_COLOR),
        ));

        // Unity TransparentStar.Awake():
        //
        // 초기 상태에서는 투명별의
        // Collider가 비활성화됩니다.
        if transparent_star.is_some() {
            entity_commands.insert(ColliderDisabled);
        }
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
