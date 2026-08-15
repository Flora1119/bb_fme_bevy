use super::{BLOCK_WORLD_SIZE, DeadlySpike, MapSpawnSet, PlayerBall, SolidBlock};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;

pub const PHYSICS_HZ: f64 = 50.0;
pub const WORLD_GRAVITY: Vec2 = Vec2::new(0.0, -9.81);

pub const MIN_BOUNCE_VELOCITY: f32 = 9.5;
pub const FLOOR_COLLISION_THRESHOLD: f32 = 0.7;
pub const CEILING_COLLISION_THRESHOLD: f32 = 0.7;

pub const PLAYER_COLLIDER_RADIUS: f32 = 0.2 * BLOCK_WORLD_SIZE;
pub const PLAYER_MASS: f32 = 5.0;
pub const PLAYER_GRAVITY_SCALE: f32 = 3.0;

pub const SOLID_COLLIDER_SIZE: Vec2 = Vec2::splat(BLOCK_WORLD_SIZE);
pub const SPIKE_SENSOR_SIZE: Vec2 = Vec2::splat(0.5 * BLOCK_WORLD_SIZE);
pub const SPIKE_SENSOR_OFFSET: Vec2 = Vec2::new(0.0, -0.25 * BLOCK_WORLD_SIZE);

const PLAYER_COLLIDER_COLOR: Color = Color::srgb(0.15, 0.80, 1.00);
const SOLID_COLLIDER_COLOR: Color = Color::srgb(0.20, 1.00, 0.35);
const SPIKE_SENSOR_COLOR: Color = Color::srgb(1.00, 0.15, 0.15);

pub struct GameplayPhysicsPlugin;

impl Plugin for GameplayPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default().with_length_unit(BLOCK_WORLD_SIZE))
            .insert_resource(Time::<Fixed>::from_hz(PHYSICS_HZ))
            .insert_resource(Gravity(WORLD_GRAVITY))
            .add_systems(
                Update,
                (attach_player_physics, attach_block_colliders)
                    .in_set(PhysicsInitializationSet)
                    .after(MapSpawnSet),
            )
            .add_systems(
                PhysicsSchedule,
                apply_solid_contact_response
                    .after(PhysicsStepSystems::Solver)
                    .before(PhysicsStepSystems::Sleeping),
            );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicsInitializationSet;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockPhysicsBody;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerPhysicsBody;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpikeSensorCollider;

#[derive(Debug, Default, Clone, Copy)]
struct StartedSolidContacts {
    floor: bool,
    ceiling: bool,
}

fn attach_player_physics(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerBall>, Without<PlayerPhysicsBody>)>,
) {
    for entity in &players {
        commands.entity(entity).insert((
            PlayerPhysicsBody,
            RigidBody::Dynamic,
            Collider::circle(PLAYER_COLLIDER_RADIUS),
            Mass(PLAYER_MASS),
            NoAutoMass,
            GravityScale(PLAYER_GRAVITY_SCALE),
            LinearDamping(0.0),
            AngularDamping(0.0),
            LockedAxes::ROTATION_LOCKED,
            Friction::ZERO,
            Restitution::ZERO,
            SweptCcd::default(),
            TransformInterpolation,
            CollisionEventsEnabled,
            DebugRender::default().with_collider_color(PLAYER_COLLIDER_COLOR),
        ));
    }
}

fn attach_block_colliders(
    mut commands: Commands,
    solids: Query<Entity, (With<SolidBlock>, Without<BlockPhysicsBody>)>,
    spikes: Query<Entity, (With<DeadlySpike>, Without<BlockPhysicsBody>)>,
) {
    for entity in &solids {
        commands.entity(entity).insert((
            BlockPhysicsBody,
            RigidBody::Static,
            Collider::rectangle(SOLID_COLLIDER_SIZE.x, SOLID_COLLIDER_SIZE.y),
            DebugRender::default().with_collider_color(SOLID_COLLIDER_COLOR),
        ));
    }

    for entity in &spikes {
        commands
            .entity(entity)
            .insert((BlockPhysicsBody, RigidBody::Static));

        commands.spawn((
            Name::new("Collider: deadly spike sensor"),
            SpikeSensorCollider,
            Sensor,
            CollisionEventsEnabled,
            Collider::rectangle(SPIKE_SENSOR_SIZE.x, SPIKE_SENSOR_SIZE.y),
            Transform::from_translation(SPIKE_SENSOR_OFFSET.extend(0.0)),
            DebugRender::default().with_collider_color(SPIKE_SENSOR_COLOR),
            ChildOf(entity),
        ));
    }
}

fn apply_solid_contact_response(
    mut collision_starts: MessageReader<CollisionStart>,
    collisions: Collisions,
    gravity: Res<Gravity>,
    players: Query<(), With<PlayerBall>>,
    solids: Query<(), With<SolidBlock>>,
    mut velocities: Query<&mut LinearVelocity, With<PlayerBall>>,
) {
    // 현재 중력의 반대 방향이 공이 튀어 오르는 방향입니다.
    //
    // 지금은 아래 방향 중력만 사용하지만, Gravity Resource를 읽도록
    // 만들어 두면 나중의 중력 반전에서도 같은 코드를 사용할 수 있습니다.
    let bounce_direction = (-gravity.0).normalize_or_zero();

    if bounce_direction == Vec2::ZERO {
        return;
    }

    // 같은 물리 틱에 여러 블록과 접촉하더라도,
    // 공마다 접촉 결과를 한 번만 처리하기 위해 먼저 집계합니다.
    let mut contacts_by_player = HashMap::<Entity, StartedSolidContacts>::new();

    for event in collision_starts.read() {
        // CollisionStart에는 접촉 법선이 없으므로
        // 실제 ContactPair에서 Manifold 정보를 읽습니다.
        let Some(contact_pair) = collisions.get(event.collider1, event.collider2) else {
            continue;
        };

        // Collider가 Rigidbody와 같은 Entity일 수도 있고
        // 자식 Entity일 수도 있으므로 Body를 우선 사용합니다.
        let body1 = contact_pair.body1.unwrap_or(contact_pair.collider1);

        let body2 = contact_pair.body2.unwrap_or(contact_pair.collider2);

        // ContactManifold의 normal은 첫 번째 물체에서
        // 두 번째 물체를 향합니다.
        //
        // normal_toward_player는 항상 표면에서 공 쪽을 향하도록
        // 방향을 통일합니다.
        let (player, normal_sign) = if players.contains(body1) && solids.contains(body2) {
            (body1, -1.0)
        } else if players.contains(body2) && solids.contains(body1) {
            (body2, 1.0)
        } else {
            // 공과 일반 SolidBlock의 접촉이 아닙니다.
            continue;
        };

        let started_contacts = contacts_by_player.entry(player).or_default();

        for manifold in &contact_pair.manifolds {
            let normal_toward_player = manifold.normal * normal_sign;

            let floor_dot = normal_toward_player.dot(bounce_direction);

            // Unity 원본과 동일하게 바닥을 우선 분류합니다.
            if floor_dot > FLOOR_COLLISION_THRESHOLD {
                started_contacts.floor = true;
            } else if floor_dot < -CEILING_COLLISION_THRESHOLD {
                started_contacts.ceiling = true;
            }
        }
    }

    for (player, started_contacts) in contacts_by_player {
        let Ok(mut velocity) = velocities.get_mut(player) else {
            continue;
        };

        let current_bounce_speed = velocity.0.dot(bounce_direction);

        // 1. 바닥 접촉은 가장 높은 우선순위를 가집니다.
        if started_contacts.floor {
            if current_bounce_speed < MIN_BOUNCE_VELOCITY {
                velocity.0 += bounce_direction * (MIN_BOUNCE_VELOCITY - current_bounce_speed);
            }

            continue;
        }

        // 2. 천장 접촉 시에는 천장 방향으로 진행하던 속도 성분만
        // 제거합니다. 수평 속도는 그대로 보존합니다.
        if started_contacts.ceiling && current_bounce_speed > 0.0 {
            velocity.0 -= bounce_direction * current_bounce_speed;
        }
    }
}
