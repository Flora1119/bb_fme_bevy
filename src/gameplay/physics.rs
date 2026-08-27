use super::{
    BLOCK_WORLD_SIZE, BlockIdentity, ClockBlock, ConsumedFunctionBlock, CurrentGridPosition,
    DeadlySpike, JumpBlock, MapSpawnSet, OneShotFunctionBlock, PlayInteractionSet, PlaySession,
    PlayerBall, SolidBlock, StraightBlock, StraightMomentum, StraightMovement,
    solid_collider_geometry_for, spike_collider_profile_for,
};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;

pub const PHYSICS_HZ: f64 = 50.0;
pub const WORLD_GRAVITY: Vec2 = Vec2::new(0.0, -9.81);

pub const MIN_BOUNCE_VELOCITY: f32 = 9.5;
pub const FLOOR_COLLISION_THRESHOLD: f32 = std::f32::consts::FRAC_1_SQRT_2;
pub const CEILING_COLLISION_THRESHOLD: f32 = 0.7;
pub const WALL_COLLISION_THRESHOLD: f32 = 0.9;
pub const MIN_WALL_BOUNCE_SPEED: f32 = 3.0;
pub const WALL_BOUNCE_DAMPING_RATIO: f32 = 0.3;

// 벽을 향하는 속도가 이 값 이하라면,
// 실제 충돌이 아니라 수치 오차 또는 스치기 접촉으로 취급합니다.
const WALL_APPROACH_EPSILON: f32 = 0.01;
const FLOOR_APPROACH_EPSILON: f32 = 0.01;

pub const PLAYER_COLLIDER_RADIUS: f32 = 0.2 * BLOCK_WORLD_SIZE;
pub const PLAYER_MASS: f32 = 5.0;
pub const PLAYER_GRAVITY_SCALE: f32 = 3.0;

pub const SOLID_COLLIDER_SIZE: Vec2 = Vec2::splat(BLOCK_WORLD_SIZE);
pub const SPIKE_SENSOR_SIZE: Vec2 = Vec2::splat(0.5 * BLOCK_WORLD_SIZE);
pub const SPIKE_SENSOR_OFFSET: Vec2 = Vec2::new(0.0, -0.25 * BLOCK_WORLD_SIZE);
pub const CLOCK_SENSOR_SIZE: f32 = 0.9 * BLOCK_WORLD_SIZE;

const CLOCK_SENSOR_COLOR: Color = Color::srgb(1.00, 0.75, 0.20);
const PLAYER_COLLIDER_COLOR: Color = Color::srgb(0.15, 0.80, 1.00);
const SOLID_COLLIDER_COLOR: Color = Color::srgb(0.20, 1.00, 0.35);
const SPIKE_SENSOR_COLOR: Color = Color::srgb(1.00, 0.15, 0.15);

pub const FLOOR_CONTACT_ANGLE_DEGREES: f32 = 45.0;
pub const PRESS_LOCKED_STRAIGHT_WALL_BOUNCE_SPEED: f32 = 3.0;

fn floor_contact_threshold() -> f32 {
    FLOOR_CONTACT_ANGLE_DEGREES.to_radians().cos()
}

#[test]
fn floor_contact_threshold_is_45_degrees() {
    let threshold = floor_contact_threshold();

    let dot_44 = 44.0_f32.to_radians().cos();
    let dot_45 = 45.0_f32.to_radians().cos();
    let dot_46 = 46.0_f32.to_radians().cos();

    assert!(dot_44 >= threshold);
    assert!(dot_45 >= threshold);
    assert!(dot_46 < threshold);
}

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
                collect_solid_contacts
                    .after(PhysicsStepSystems::NarrowPhase)
                    .before(PhysicsStepSystems::Solver),
            )
            .add_systems(
                PhysicsSchedule,
                apply_solid_contact_response
                    .after(PhysicsStepSystems::Solver)
                    .after(PlayInteractionSet::Resolve)
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

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidColliderChild;

#[derive(Resource, Debug, Default)]
struct PendingSolidContactResponses(HashMap<Entity, StartedSolidContacts>);

#[derive(Debug, Clone, Copy)]
struct JumpContactResponse {
    source: Entity,
    speed: f32,
}

#[derive(Debug, Clone, Copy)]
struct StraightContactResponse {
    source: Entity,
    target_position: Vec2,
    direction: Vec2,
    speed: f32,
}

#[derive(Debug, Clone, Copy)]
struct StartedSolidContacts {
    floor: bool,
    corner_glide: bool,
    ceiling: bool,

    // 벽에서 공 쪽으로 향하는 방향입니다.
    // 즉, 벽 반대 방향이자 공이 튕겨 나갈 방향입니다.
    wall_direction: Vec2,

    // Avian Solver가 속도를 제거하기 전에 저장한 입사 속도입니다.
    incoming_velocity: Vec2,

    // 바닥으로 판정된 접촉 중 JumpBlock이 있었다면
    // 적용할 점프 속도입니다.
    jump: Option<JumpContactResponse>,
    straight: Option<StraightContactResponse>,
}

impl StartedSolidContacts {
    fn new(incoming_velocity: Vec2) -> Self {
        Self {
            floor: false,
            corner_glide: false,
            ceiling: false,
            wall_direction: Vec2::ZERO,
            incoming_velocity,
            jump: None,
            straight: None,
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
    solids: Query<
        (Entity, Option<&BlockIdentity>),
        (
            With<SolidBlock>,
            Without<DeadlySpike>,
            Without<BlockPhysicsBody>,
        ),
    >,
    spikes: Query<(Entity, Option<&BlockIdentity>), (With<DeadlySpike>, Without<BlockPhysicsBody>)>,
    clocks: Query<Entity, (With<ClockBlock>, Without<BlockPhysicsBody>)>,
) {
    // 일반 SolidBlock
    //
    // DeadlySpike도 함께 가진 복합 가시는
    // 아래 spikes 루프에서 별도로 처리합니다.
    for (entity, identity) in &solids {
        let geometry = solid_collider_geometry_for(identity.map(|identity| identity.id.as_str()));

        commands
            .entity(entity)
            .insert((BlockPhysicsBody, RigidBody::Static));

        if geometry.offset() == Vec2::ZERO {
            commands.entity(entity).insert((
                Collider::rectangle(geometry.size().x, geometry.size().y),
                DebugRender::default().with_collider_color(SOLID_COLLIDER_COLOR),
            ));
        } else {
            commands.spawn((
                Name::new("Collider: offset solid block"),
                SolidColliderChild,
                Collider::rectangle(geometry.size().x, geometry.size().y),
                Transform::from_translation(geometry.offset().extend(0.0)),
                DebugRender::default().with_collider_color(SOLID_COLLIDER_COLOR),
                ChildOf(entity),
            ));
        }
    }

    // 가시 및 가시+블록 복합체
    for (entity, identity) in &spikes {
        let profile = spike_collider_profile_for(identity.map(|identity| identity.id.as_str()));

        commands
            .entity(entity)
            .insert((BlockPhysicsBody, RigidBody::Static));

        // s_b_* 계열의 Solid 부분
        if let Some(geometry) = profile.solid() {
            commands.spawn((
                Name::new("Collider: solid spike block"),
                SolidColliderChild,
                Collider::rectangle(geometry.size().x, geometry.size().y),
                Transform::from_translation(geometry.offset().extend(0.0)),
                DebugRender::default().with_collider_color(SOLID_COLLIDER_COLOR),
                ChildOf(entity),
            ));
        }

        // 실제 Damage Trigger.
        //
        // s_b_two / s_b_o_two는
        // 여기서 2개가 생성됩니다.
        for (sensor_index, geometry) in profile.damage_sensors().iter().copied().enumerate() {
            commands.spawn((
                Name::new(format!(
                    "Collider: deadly spike sensor {}",
                    sensor_index + 1,
                )),
                SpikeSensorCollider,
                Sensor,
                CollisionEventsEnabled,
                Collider::rectangle(geometry.size().x, geometry.size().y),
                Transform::from_translation(geometry.offset().extend(0.0)),
                DebugRender::default().with_collider_color(SPIKE_SENSOR_COLOR),
                ChildOf(entity),
            ));
        }
    }

    // 방향 선택(시계) 블록
    //
    // Unity 원본과 동일하게
    // 0.9 x 0.9 Trigger 영역으로 사용합니다.
    for entity in &clocks {
        commands.entity(entity).insert((
            BlockPhysicsBody,
            RigidBody::Static,
            Sensor,
            CollisionEventsEnabled,
            Collider::rectangle(CLOCK_SENSOR_SIZE, CLOCK_SENSOR_SIZE),
            DebugRender::default().with_collider_color(CLOCK_SENSOR_COLOR),
        ));
    }
}

fn collect_solid_contacts(
    collisions: Collisions,
    gravity: Res<Gravity>,
    players: Query<&LinearVelocity, With<PlayerBall>>,
    solids: Query<(), (With<SolidBlock>, Without<ConsumedFunctionBlock>)>,
    spike_sensors: Query<(), With<SpikeSensorCollider>>,
    jump_blocks: Query<&JumpBlock>,
    straight_blocks: Query<(&StraightBlock, &CurrentGridPosition)>,
    mut pending: ResMut<PendingSolidContactResponses>,
) {
    // 이번 물리 틱의 현재 접촉 상태를 새로 수집합니다.
    pending.0.clear();

    let gravity_direction = gravity.0.normalize_or_zero();
    let bounce_direction = -gravity_direction;

    if gravity_direction == Vec2::ZERO {
        return;
    }

    // 중력에 수직인 축입니다.
    let wall_axis = Vec2::new(bounce_direction.y, -bounce_direction.x);

    // CollisionStart만 보는 대신,
    // 현재 실제로 접촉 중인 모든 pair를 매 물리 틱 검사합니다.
    for contact_pair in collisions.iter() {
        // 복합 가시의 Damage Sensor는 부모가 SolidBlock이더라도
        // 실제 물리 표면이 아닙니다.
        //
        // 따라서 Solid 접촉 응답에서는 완전히 제외합니다.
        if spike_sensors.contains(contact_pair.collider1)
            || spike_sensors.contains(contact_pair.collider2)
        {
            continue;
        }

        let body1 = contact_pair.body1.unwrap_or(contact_pair.collider1);

        let body2 = contact_pair.body2.unwrap_or(contact_pair.collider2);

        let (player, solid, normal_sign, player_is_first) =
            if players.contains(body1) && solids.contains(body2) {
                (body1, body2, -1.0, true)
            } else if players.contains(body2) && solids.contains(body1) {
                (body2, body1, 1.0, false)
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
            let normal_toward_player = manifold.normal * normal_sign;

            let normal_floor_dot = normal_toward_player.dot(bounce_direction);

            let wall_dot = normal_toward_player.dot(wall_axis);

            let is_clear_wall = wall_dot.abs() > WALL_COLLISION_THRESHOLD;

            // 공 중심 A -> 실제 접촉점 K
            let mut lowest_floor_alignment: Option<f32> = None;

            for point in &manifold.points {
                let player_anchor = if player_is_first {
                    point.anchor1
                } else {
                    point.anchor2
                };

                let contact_direction = player_anchor.normalize_or_zero();

                if contact_direction == Vec2::ZERO {
                    continue;
                }

                // GeoGebra에서 정한 ∠KAB 판정입니다.
                //
                // A -> K : contact_direction
                // A -> B : gravity_direction
                let floor_alignment = contact_direction.dot(gravity_direction);

                lowest_floor_alignment = Some(
                    lowest_floor_alignment
                        .map_or(floor_alignment, |current| current.min(floor_alignment)),
                );
            }

            let Some(contact_floor_alignment) = lowest_floor_alignment else {
                continue;
            };

            let floor_threshold = floor_contact_threshold();

            // 1. 45도 이하: 바닥
            if contact_floor_alignment >= floor_threshold {
                started_contacts.floor = true;

                if let Ok((straight_block, grid_position)) = straight_blocks.get(solid) {
                    let grid = grid_position.0;

                    let block_center = Vec2::new(
                        grid.x as f32 * BLOCK_WORLD_SIZE,
                        grid.y as f32 * BLOCK_WORLD_SIZE,
                    );

                    let candidate = StraightContactResponse {
                        source: solid,
                        target_position: block_center
                            + straight_block.exit_offset() * BLOCK_WORLD_SIZE,
                        direction: straight_block.launch_direction(),
                        speed: straight_block.speed(),
                    };

                    let should_replace = match started_contacts.straight {
                        Some(current) => candidate.speed > current.speed,
                        None => true,
                    };

                    if should_replace {
                        started_contacts.straight = Some(candidate);
                    }
                }

                if let Ok(jump_block) = jump_blocks.get(solid) {
                    let candidate = JumpContactResponse {
                        source: solid,
                        speed: jump_block.launch_speed(),
                    };

                    let should_replace = match started_contacts.jump {
                        Some(current) => candidate.speed > current.speed,
                        None => true,
                    };

                    if should_replace {
                        started_contacts.jump = Some(candidate);
                    }
                }

                continue;
            }

            // 2. 공의 아래쪽이지만 45도 초과:
            // 모서리 glide
            //
            // 단, 접촉면의 normal이 명백한 벽이라면
            // 접촉점이 수치 오차로 조금 아래쪽에 잡혔더라도
            // corner glide로 오인하지 않습니다.
            if contact_floor_alignment > 0.0 && !is_clear_wall {
                started_contacts.corner_glide = true;

                continue;
            }

            // 3. 천장
            if normal_floor_dot < -CEILING_COLLISION_THRESHOLD {
                started_contacts.ceiling = true;

                continue;
            }

            // 4. 벽
            if is_clear_wall {
                let candidate_direction = wall_axis * wall_dot.signum();

                let candidate_impact_speed =
                    -started_contacts.incoming_velocity.dot(candidate_direction);

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

fn apply_solid_contact_response(
    mut commands: Commands,
    gravity: Res<Gravity>,
    session: Option<Res<PlaySession>>,
    one_shot_blocks: Query<(), With<OneShotFunctionBlock>>,
    mut pending: ResMut<PendingSolidContactResponses>,
    mut players: Query<
        (
            &mut LinearVelocity,
            &mut GravityScale,
            &mut Position,
            &mut Transform,
            Option<&StraightMovement>,
            Option<&StraightMomentum>,
        ),
        With<PlayerBall>,
    >,
) {
    let bounce_direction = (-gravity.0).normalize_or_zero();

    if bounce_direction == Vec2::ZERO {
        pending.0.clear();
        return;
    }

    let gameplay_is_playing = session
        .as_ref()
        .map_or(true, |session| session.is_playing());

    let contacts_by_player = std::mem::take(&mut pending.0);

    for (player, started_contacts) in contacts_by_player {
        let Ok((
            mut velocity,
            mut gravity_scale,
            mut position,
            mut transform,
            straight_movement,
            straight_momentum,
        )) = players.get_mut(player)
        else {
            continue;
        };

        let was_press_locked_straight =
            straight_movement.is_some_and(|straight| !straight.can_cancel_on_press());

        let current_bounce_speed = velocity.0.dot(bounce_direction);

        let incoming_bounce_speed = started_contacts.incoming_velocity.dot(bounce_direction);

        // Unity에서는 직진 중
        // 바닥/천장/벽에 부딪히면
        // StraightMovement가 종료되고
        // 중력이 다시 켜집니다.
        //
        // corner_glide는 실제 blocking
        // collision으로 취급하지 않습니다.
        let blocking_contact = started_contacts.floor
            || started_contacts.ceiling
            || started_contacts.wall_direction != Vec2::ZERO;

        if straight_movement.is_some() && blocking_contact {
            *gravity_scale = GravityScale(PLAYER_GRAVITY_SCALE);

            commands.entity(player).remove::<StraightMovement>();
        }

        if straight_momentum.is_some() && blocking_contact {
            commands.entity(player).remove::<StraightMomentum>();
        }

        if started_contacts.corner_glide {
            continue;
        }

        // 1. 바닥 접촉
        if started_contacts.floor {
            if incoming_bounce_speed > FLOOR_APPROACH_EPSILON {
                continue;
            }

            if gameplay_is_playing {
                // StraightBlock
                //
                // JumpBlock보다 먼저 처리하여
                // 직진 블록이 정확한
                // 위치와 속도를 최종 결정합니다.
                if let Some(straight) = started_contacts.straight {
                    position.0 = straight.target_position;

                    transform.translation.x = straight.target_position.x;

                    transform.translation.y = straight.target_position.y;

                    velocity.0 = straight.direction * straight.speed;

                    *gravity_scale = GravityScale(0.0);

                    commands
                        .entity(player)
                        .insert(StraightMovement::new(straight.direction, straight.speed));

                    if one_shot_blocks.contains(straight.source) {
                        commands.entity(straight.source).insert((
                            ConsumedFunctionBlock,
                            Visibility::Hidden,
                            ColliderDisabled,
                        ));
                    }

                    continue;
                }

                // JumpBlock
                if let Some(jump) = started_contacts.jump {
                    velocity.0 += bounce_direction * (jump.speed - current_bounce_speed);

                    if one_shot_blocks.contains(jump.source) {
                        commands.entity(jump.source).insert((
                            ConsumedFunctionBlock,
                            Visibility::Hidden,
                            ColliderDisabled,
                        ));
                    }

                    continue;
                }
            }

            // 일반 블록
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
            let incoming_wall_speed = -started_contacts
                .incoming_velocity
                .dot(started_contacts.wall_direction);

            let outgoing_wall_speed = if was_press_locked_straight {
                PRESS_LOCKED_STRAIGHT_WALL_BOUNCE_SPEED
            } else {
                (incoming_wall_speed * WALL_BOUNCE_DAMPING_RATIO).max(MIN_WALL_BOUNCE_SPEED)
            };

            let current_wall_speed = velocity.0.dot(started_contacts.wall_direction);

            velocity.0 -= started_contacts.wall_direction * current_wall_speed;

            velocity.0 += started_contacts.wall_direction * outgoing_wall_speed;
        }
    }
}
