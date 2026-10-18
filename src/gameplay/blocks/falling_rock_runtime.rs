use super::{FallingRockCollisionPlugin, FallingRockSpawner};
use crate::gameplay::{
    ActivePlayWorld, BLOCK_WORLD_SIZE, MapSpawnSet, PlaySession, PlaySessionSet, PlayerBall,
    RuntimeBlock, TransportAnimationSet,
};
use avian2d::prelude::*;
use bevy::prelude::*;

const FALLING_ROCK_IMAGE: &str = "sprites/funcblock/fb_fr.png";

pub const FALLING_ROCK_FADE_SECONDS: f64 = 0.5;
pub const FALLING_ROCK_WAIT_SECONDS: f64 = 0.2;

const TIME_EPSILON: f64 = 0.000_001;

pub struct FallingRockPlugin;

impl Plugin for FallingRockPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FallingRockCollisionPlugin);
        app.add_systems(Update, initialize_falling_rock_spawners.after(MapSpawnSet))
            .add_systems(
                PhysicsSchedule,
                (advance_falling_rocks, spawn_due_falling_rocks)
                    .chain()
                    .after(PhysicsStepSystems::First)
                    .after(PlaySessionSet::AdvanceTime)
                    .after(TransportAnimationSet)
                    .before(PhysicsStepSystems::BroadPhase),
            );
    }
}

/// 맵에 배치된 생성기와 별도로 존재하는 실제 낙석입니다.
#[derive(Component, Debug, Default)]
pub struct FallingRock {
    age_seconds: f64,
}

impl FallingRock {
    pub fn is_falling(&self) -> bool {
        self.age_seconds + TIME_EPSILON >= FALLING_ROCK_FADE_SECONDS + FALLING_ROCK_WAIT_SECONDS
    }

    pub fn alpha(&self) -> f32 {
        (self.age_seconds / FALLING_ROCK_FADE_SECONDS).clamp(0.0, 1.0) as f32
    }

    fn advance(&mut self, delta_seconds: f64) {
        if delta_seconds.is_finite() && delta_seconds > 0.0 {
            self.age_seconds += delta_seconds;
        }
    }
}

#[derive(Component, Debug)]
struct FallingRockSpawnClock {
    remaining_seconds: f64,
    finished: bool,
}

impl FallingRockSpawnClock {
    fn new(delay_seconds: f32) -> Self {
        Self {
            remaining_seconds: f64::from(delay_seconds),
            finished: false,
        }
    }

    fn advance(&mut self, delta_seconds: f64, interval_seconds: f32) -> bool {
        if self.finished || !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return false;
        }

        self.remaining_seconds -= delta_seconds;

        if self.remaining_seconds > TIME_EPSILON {
            return false;
        }

        if interval_seconds <= 0.0 {
            // 현재 맵 설정은 양수만 허용하지만,
            // 원본 생성기의 한 번만 생성하는 조건도 유지합니다.
            self.finished = true;
        } else {
            // 원본 코루틴처럼 이번 생성 시점부터 다음 간격을 셉니다.
            // 긴 프레임에서도 밀린 낙석을 한꺼번에 생성하지 않습니다.
            self.remaining_seconds = f64::from(interval_seconds);
        }

        true
    }
}

fn initialize_falling_rock_spawners(
    mut commands: Commands,
    spawners: Query<(Entity, &FallingRockSpawner), Without<FallingRockSpawnClock>>,
) {
    for (entity, spawner) in &spawners {
        commands
            .entity(entity)
            .insert(FallingRockSpawnClock::new(spawner.delay_seconds()));
    }
}

fn spawn_due_falling_rocks(
    mut commands: Commands,
    time: Res<Time<Physics>>,
    session: Res<PlaySession>,
    active_world: Res<ActivePlayWorld>,
    asset_server: Option<Res<AssetServer>>,
    mut spawners: Query<(
        &FallingRockSpawner,
        &mut FallingRockSpawnClock,
        &Transform,
        Option<&Position>,
    )>,
) {
    if !session.is_playing() {
        return;
    }

    let Some(root) = active_world.root() else {
        return;
    };

    for (spawner, mut clock, transform, position) in &mut spawners {
        if !clock.advance(time.delta_secs_f64(), spawner.interval_seconds()) {
            continue;
        }

        // 밀기·당기기로 생성기가 이동 중이라면 현재 실제 위치를 사용합니다.
        let spawn_position =
            position.map_or(transform.translation.truncate(), |position| position.0);

        let image: Handle<Image> = asset_server
            .as_ref()
            .map(|server| server.load(FALLING_ROCK_IMAGE))
            .unwrap_or_default();

        commands.spawn((
            Name::new("Falling Rock"),
            FallingRock::default(),
            Sprite {
                image,
                custom_size: Some(Vec2::splat(BLOCK_WORLD_SIZE)),
                color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                ..default()
            },
            Transform::from_translation(spawn_position.extend(0.01)),
            Position(spawn_position),
            RigidBody::Dynamic,
            (
                Mass(1.0),
                AngularInertia(BLOCK_WORLD_SIZE * BLOCK_WORLD_SIZE / 6.0),
                NoAutoMass,
                NoAutoAngularInertia,
            ),
            Collider::rectangle(BLOCK_WORLD_SIZE, BLOCK_WORLD_SIZE),
            Sensor,
            CollisionEventsEnabled,
            GravityScale(0.0),
            LinearVelocity(Vec2::ZERO),
            LockedAxes::ROTATION_LOCKED,
            // 숨겨진 생성기의 자식으로 만들면 낙석도 숨겨집니다.
            // 맵 루트에 소속시켜 표시하고 재시작 시 함께 제거합니다.
            ChildOf(root),
        ));
    }
}

fn advance_falling_rocks(
    time: Res<Time<Physics>>,
    session: Res<PlaySession>,
    mut rocks: Query<
        (
            &mut FallingRock,
            &mut GravityScale,
            &mut LinearVelocity,
            &mut Sprite,
        ),
        (Without<PlayerBall>, Without<RuntimeBlock>),
    >,
) {
    for (mut rock, mut gravity_scale, mut velocity, mut sprite) in &mut rocks {
        if !session.is_playing() {
            gravity_scale.0 = 0.0;
            velocity.0 = Vec2::ZERO;
            continue;
        }

        rock.advance(time.delta_secs_f64());
        sprite.color.set_alpha(rock.alpha());

        if rock.is_falling() {
            // 프리팹의 배율 6이 아니라, 원본 스크립트가 지정한 1입니다.
            gravity_scale.0 = 1.0;
        } else {
            gravity_scale.0 = 0.0;
            velocity.0 = Vec2::ZERO;
        }
    }
}

#[cfg(test)]
mod falling_rock_runtime_tests {
    use super::*;

    #[test]
    fn falling_rock_fades_then_waits_before_falling() {
        let mut rock = FallingRock::default();

        assert_eq!(rock.alpha(), 0.0);
        assert!(!rock.is_falling());

        for _ in 0..25 {
            rock.advance(0.02);
        }

        assert!((rock.alpha() - 1.0).abs() < 0.000_001);
        assert!(!rock.is_falling());

        for _ in 0..9 {
            rock.advance(0.02);
        }

        // 0.68초에는 아직 대기 중입니다.
        assert!(!rock.is_falling());

        rock.advance(0.02);

        // 0.70초부터 낙하합니다.
        assert!(rock.is_falling());
        assert_eq!(rock.alpha(), 1.0);
    }

    #[test]
    fn falling_rock_spawn_clock_respects_delay_and_interval() {
        let mut clock = FallingRockSpawnClock::new(0.5);

        for _ in 0..24 {
            assert!(!clock.advance(0.02, 1.0));
        }

        assert!(clock.advance(0.02, 1.0));

        for _ in 0..49 {
            assert!(!clock.advance(0.02, 1.0));
        }

        assert!(clock.advance(0.02, 1.0));
    }

    #[test]
    fn falling_rock_zero_delay_spawns_on_first_tick() {
        let mut clock = FallingRockSpawnClock::new(0.0);

        assert!(clock.advance(0.02, 1.0));
        assert!(!clock.advance(0.02, 1.0));
    }

    #[test]
    fn falling_rock_non_positive_interval_spawns_only_once() {
        for interval in [0.0, -1.0] {
            let mut clock = FallingRockSpawnClock::new(0.0);

            assert!(clock.advance(0.02, interval));
            assert!(!clock.advance(10.0, interval));
        }
    }

    #[test]
    fn falling_rock_long_tick_does_not_create_a_backlog() {
        let mut clock = FallingRockSpawnClock::new(0.5);

        assert!(clock.advance(5.0, 1.0));
        assert!(!clock.advance(0.0, 1.0));
        assert!(!clock.advance(0.5, 1.0));
        assert!(clock.advance(0.5, 1.0));
    }
}
