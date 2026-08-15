use super::{BLOCK_WORLD_SIZE, DeadlySpike, MapSpawnSet, PlayerBall, SolidBlock};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;

pub const PHYSICS_HZ: f64 = 50.0;
pub const WORLD_GRAVITY: Vec2 = Vec2::new(0.0, -9.81);

pub const MIN_BOUNCE_VELOCITY: f32 = 9.5;
pub const FLOOR_COLLISION_THRESHOLD: f32 = 0.7;
pub const CEILING_COLLISION_THRESHOLD: f32 = 0.7;
pub const WALL_COLLISION_THRESHOLD: f32 = 0.9;
pub const MIN_WALL_BOUNCE_SPEED: f32 = 12.0;
pub const WALL_BOUNCE_DAMPING_RATIO: f32 = 0.8;

// 벽을 향하는 속도가 이 값 이하라면,
// 실제 충돌이 아니라 수치 오차 또는 스치기 접촉으로 취급합니다.
const WALL_APPROACH_EPSILON: f32 = 0.01;

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
            .init_resource::<PendingSolidContactResponses>()
            .add_systems(
                Update,
                (attach_player_physics, attach_block_colliders)
                    .in_set(PhysicsInitializationSet)
                    .after(MapSpawnSet),
            )
            .add_systems(
                PhysicsSchedule,
                collect_started_solid_contacts
                    .after(PhysicsStepSystems::NarrowPhase)
                    .before(PhysicsStepSystems::Solver),
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

#[derive(Resource, Debug, Default)]
struct PendingSolidContactResponses(HashMap<Entity, StartedSolidContacts>);

#[derive(Debug, Clone, Copy)]
struct StartedSolidContacts {
    floor: bool,
    ceiling: bool,

    // 벽에서 공 쪽으로 향하는 방향입니다.
    // 즉, 벽 반대 방향이자 공이 튕겨 나갈 방향입니다.
    wall_direction: Vec2,

    // Avian Solver가 속도를 제거하기 전에 저장한 입사 속도입니다.
    incoming_velocity: Vec2,
}

impl StartedSolidContacts {
    fn new(incoming_velocity: Vec2) -> Self {
        Self {
            floor: false,
            ceiling: false,
            wall_direction: Vec2::ZERO,
            incoming_velocity,
        }
    }
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

fn collect_started_solid_contacts(
    mut collision_starts: MessageReader<CollisionStart>,
    collisions: Collisions,
    gravity: Res<Gravity>,
    players: Query<&LinearVelocity, With<PlayerBall>>,
    solids: Query<(), With<SolidBlock>>,
    mut pending: ResMut<PendingSolidContactResponses>,
) {
    // 이전 물리 틱에 남은 임시 결과를 제거합니다.
    pending.0.clear();

    let bounce_direction = (-gravity.0).normalize_or_zero();

    if bounce_direction == Vec2::ZERO {
        return;
    }

    // 중력에 수직인 축입니다.
    //
    // 현재 아래 방향 중력에서는 Vec2::X와 같으며,
    // 중력이 반전돼도 벽 반동 계산에 그대로 사용할 수 있습니다.
    let wall_axis = Vec2::new(bounce_direction.y, -bounce_direction.x);

    for event in collision_starts.read() {
        let Some(contact_pair) = collisions.get(event.collider1, event.collider2) else {
            continue;
        };

        let body1 = contact_pair.body1.unwrap_or(contact_pair.collider1);

        let body2 = contact_pair.body2.unwrap_or(contact_pair.collider2);

        let (player, normal_sign) = if players.contains(body1) && solids.contains(body2) {
            (body1, -1.0)
        } else if players.contains(body2) && solids.contains(body1) {
            (body2, 1.0)
        } else {
            continue;
        };

        let Ok(player_velocity) = players.get(player) else {
            continue;
        };

        let started_contacts = pending
            .0
            .entry(player)
            .or_insert_with(|| StartedSolidContacts::new(player_velocity.0));

        for manifold in &contact_pair.manifolds {
            // 표면에서 공 쪽을 향하는 법선으로 방향을 통일합니다.
            let normal_toward_player = manifold.normal * normal_sign;

            let floor_dot = normal_toward_player.dot(bounce_direction);

            // Unity와 동일하게 한 Manifold를
            // 바닥 → 천장 → 벽 순서로 분류합니다.
            if floor_dot > FLOOR_COLLISION_THRESHOLD {
                started_contacts.floor = true;
            } else if floor_dot < -CEILING_COLLISION_THRESHOLD {
                started_contacts.ceiling = true;
            } else {
                let wall_dot = normal_toward_player.dot(wall_axis);

                if wall_dot.abs() > WALL_COLLISION_THRESHOLD {
                    // wall_axis 위에서 실제 법선 방향을 복원합니다.
                    let candidate_direction = wall_axis * wall_dot.signum();

                    let candidate_impact_speed =
                        -started_contacts.incoming_velocity.dot(candidate_direction);

                    // 양수일 때만 공이 해당 벽을 향해 접근 중입니다.
                    //
                    // 0에 가깝다면 벽을 평행하게 스치는 중이고,
                    // 음수라면 이미 벽에서 멀어지는 중입니다.
                    if candidate_impact_speed <= WALL_APPROACH_EPSILON {
                        continue;
                    }

                    let current_impact_speed = if started_contacts.wall_direction == Vec2::ZERO {
                        f32::NEG_INFINITY
                    } else {
                        -started_contacts
                            .incoming_velocity
                            .dot(started_contacts.wall_direction)
                    };

                    if candidate_impact_speed > current_impact_speed {
                        started_contacts.wall_direction = candidate_direction;
                    }
                }
            }
        }
    }
}

fn apply_solid_contact_response(
    gravity: Res<Gravity>,
    mut pending: ResMut<PendingSolidContactResponses>,
    mut velocities: Query<&mut LinearVelocity, With<PlayerBall>>,
) {
    let bounce_direction = (-gravity.0).normalize_or_zero();

    if bounce_direction == Vec2::ZERO {
        pending.0.clear();
        return;
    }

    // 이번 물리 틱의 결과를 꺼내고 Resource는 빈 상태로 만듭니다.
    let contacts_by_player = std::mem::take(&mut pending.0);

    for (player, started_contacts) in contacts_by_player {
        let Ok(mut velocity) = velocities.get_mut(player) else {
            continue;
        };

        let current_bounce_speed = velocity.0.dot(bounce_direction);

        // 1. 바닥 접촉
        if started_contacts.floor {
            if current_bounce_speed < MIN_BOUNCE_VELOCITY {
                velocity.0 += bounce_direction * (MIN_BOUNCE_VELOCITY - current_bounce_speed);
            }

            continue;
        }

        // 2. 천장 접촉
        if started_contacts.ceiling {
            if current_bounce_speed > 0.0 {
                velocity.0 -= bounce_direction * current_bounce_speed;
            }

            continue;
        }

        // 3. 벽 접촉
        if started_contacts.wall_direction != Vec2::ZERO {
            // 선택된 벽을 향하던 실제 접근 속력입니다.
            //
            // collect_started_solid_contacts에서 양수임을 이미
            // 확인했으므로 절댓값으로 방향 정보를 잃지 않습니다.
            let incoming_wall_speed = -started_contacts
                .incoming_velocity
                .dot(started_contacts.wall_direction);

            let outgoing_wall_speed =
                (incoming_wall_speed * WALL_BOUNCE_DAMPING_RATIO).max(MIN_WALL_BOUNCE_SPEED);

            // Solver가 남긴 벽 법선 방향의 속도만 제거합니다.
            // 중력 방향과 평행한 속도는 그대로 보존합니다.
            let current_wall_speed = velocity.0.dot(started_contacts.wall_direction);

            velocity.0 -= started_contacts.wall_direction * current_wall_speed;

            // 벽에서 공 쪽으로 반동합니다.
            velocity.0 += started_contacts.wall_direction * outgoing_wall_speed;
        }
    }
}
