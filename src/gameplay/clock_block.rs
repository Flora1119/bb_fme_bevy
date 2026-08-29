use super::{
    BLOCK_WORLD_SIZE, BlockFacing, BlockVisualSet, CurrentGridPosition, PendingPlayInteractions,
    PlayInteraction, PlayInteractionCollectSet, PlayInteractionSet, PlaySession, PlaySessionSet,
    PlayerBall, PlayerControlInputSet, ResolvedMovementInteraction, StraightMomentum,
    StraightMovement,
};
use avian2d::prelude::*;
use bevy::{input::InputSystems, prelude::*};

const CLOCK_ARROW_ASSET_PATH: &str = "sprites/funcblock/fb_clock_arrow.png";

const CLOCK_ARROW_Z: f32 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockDirectionMode {
    Dir4,
    Dir8,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockBlock {
    mode: ClockDirectionMode,
}

impl ClockBlock {
    pub const ROTATE_INTERVAL_SECONDS: f32 = 0.3;
    pub const LAUNCH_SPEED: f32 = 15.0;
    pub const LAUNCH_OFFSET_BLOCKS: f32 = 0.5;

    pub const fn dir4() -> Self {
        Self {
            mode: ClockDirectionMode::Dir4,
        }
    }

    pub const fn dir8() -> Self {
        Self {
            mode: ClockDirectionMode::Dir8,
        }
    }

    pub const fn mode(self) -> ClockDirectionMode {
        self.mode
    }

    pub const fn direction_count(self) -> u8 {
        match self.mode {
            ClockDirectionMode::Dir4 => 4,
            ClockDirectionMode::Dir8 => 8,
        }
    }

    pub const fn rotation_step_degrees(self) -> f32 {
        match self.mode {
            ClockDirectionMode::Dir4 => 90.0,
            ClockDirectionMode::Dir8 => 45.0,
        }
    }

    pub fn launch_direction(self, direction_index: u8) -> Vec2 {
        const D: f32 = std::f32::consts::FRAC_1_SQRT_2;

        match self.mode {
            ClockDirectionMode::Dir4 => match direction_index % 4 {
                0 => Vec2::new(0.0, 1.0),
                1 => Vec2::new(1.0, 0.0),
                2 => Vec2::new(0.0, -1.0),
                3 => Vec2::new(-1.0, 0.0),
                _ => unreachable!(),
            },
            ClockDirectionMode::Dir8 => match direction_index % 8 {
                0 => Vec2::new(0.0, 1.0),
                1 => Vec2::new(D, D),
                2 => Vec2::new(1.0, 0.0),
                3 => Vec2::new(D, -D),
                4 => Vec2::new(0.0, -1.0),
                5 => Vec2::new(-D, -D),
                6 => Vec2::new(-1.0, 0.0),
                7 => Vec2::new(-D, D),
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ClockSelection {
    source: Entity,
    direction_index: u8,
    elapsed_seconds: f32,
}

impl ClockSelection {
    pub const fn new(source: Entity) -> Self {
        Self {
            source,
            direction_index: 0,
            elapsed_seconds: 0.0,
        }
    }

    pub const fn source(self) -> Entity {
        self.source
    }

    pub const fn direction_index(self) -> u8 {
        self.direction_index
    }

    pub const fn elapsed_seconds(self) -> f32 {
        self.elapsed_seconds
    }

    pub fn advance(&mut self, delta_seconds: f32, clock: ClockBlock) -> u32 {
        const TIME_EPSILON_SECONDS: f32 = 0.000_001;

        self.elapsed_seconds += delta_seconds.max(0.0);

        let mut rotations = 0;

        while self.elapsed_seconds + TIME_EPSILON_SECONDS >= ClockBlock::ROTATE_INTERVAL_SECONDS {
            self.elapsed_seconds =
                (self.elapsed_seconds - ClockBlock::ROTATE_INTERVAL_SECONDS).max(0.0);

            self.direction_index = (self.direction_index + 1) % clock.direction_count();

            rotations += 1;
        }

        rotations
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockLaunchGuard {
    source: Entity,
}

impl ClockLaunchGuard {
    pub const fn new(source: Entity) -> Self {
        Self { source }
    }

    pub const fn source(self) -> Entity {
        self.source
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct ClockArrowVisual {
    source: Entity,
}

pub struct ClockBlockPlugin;

impl Plugin for ClockBlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_clock_arrow_visuals, sync_clock_arrow_visuals)
                .chain()
                .after(BlockVisualSet),
        )
        .add_systems(
            PhysicsSchedule,
            (
                collect_started_clock_interactions,
                clear_clock_launch_guard_on_exit,
            )
                .chain()
                .in_set(PlayInteractionSet::Collect)
                .in_set(PlayInteractionCollectSet::Movement),
        )
        .add_systems(
            PhysicsSchedule,
            advance_clock_selection
                .after(PlaySessionSet::AdvanceTime)
                .before(PhysicsStepSystems::BroadPhase),
        )
        .add_systems(
            PhysicsSchedule,
            activate_resolved_clock_selection
                .after(PhysicsStepSystems::Sleeping)
                .before(PhysicsStepSystems::Finalize),
        )
        .add_systems(
            PreUpdate,
            launch_clock_selection_on_press
                .after(InputSystems)
                .after(PlayerControlInputSet),
        );
    }
}

fn collect_started_clock_interactions(
    mut collision_starts: MessageReader<CollisionStart>,

    players: Query<Option<&ClockLaunchGuard>, (With<PlayerBall>, Without<ClockSelection>)>,

    clocks: Query<(), With<ClockBlock>>,

    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let (player, clock) =
            if players.contains(event.collider1) && clocks.contains(event.collider2) {
                (event.collider1, event.collider2)
            } else if players.contains(event.collider2) && clocks.contains(event.collider1) {
                (event.collider2, event.collider1)
            } else {
                continue;
            };

        let Ok(guard) = players.get(player) else {
            continue;
        };

        // 방금 발사되어 아직 같은 Clock Sensor 안에 있다면
        // 재진입으로 취급하지 않습니다.
        if guard.is_some_and(|guard| guard.source() == clock) {
            continue;
        }

        pending.push(PlayInteraction::movement(clock, player));
    }
}

fn activate_resolved_clock_selection(
    mut commands: Commands,
    mut resolved_movements: MessageReader<ResolvedMovementInteraction>,

    clocks: Query<&CurrentGridPosition, With<ClockBlock>>,

    mut players: Query<
        (
            &mut Position,
            &mut Transform,
            &mut LinearVelocity,
            &mut GravityScale,
        ),
        With<PlayerBall>,
    >,
) {
    for movement in resolved_movements.read() {
        let source = movement.source();
        let player = movement.actor();

        let Ok(grid_position) = clocks.get(source) else {
            // 다른 종류의 Movement 블록이 나중에 추가되어도
            // ClockBlock 시스템은 자기 것만 처리합니다.
            continue;
        };

        let Ok((mut position, mut transform, mut velocity, mut gravity_scale)) =
            players.get_mut(player)
        else {
            continue;
        };

        let grid = grid_position.0;

        let center = Vec2::new(
            grid.x as f32 * BLOCK_WORLD_SIZE,
            grid.y as f32 * BLOCK_WORLD_SIZE,
        );

        // Unity ClockBlock.ActivateDirectionSelect():
        //
        // 플레이어를 블록 중앙에 정확히 고정합니다.
        position.0 = center;
        transform.translation.x = center.x;
        transform.translation.y = center.y;

        // 기존 움직임 완전 정지.
        velocity.0 = Vec2::ZERO;

        // 선택 중에는 중력을 받지 않습니다.
        *gravity_scale = GravityScale(0.0);

        commands
            .entity(player)
            // Unity RigidbodyType2D.Kinematic 대응.
            .insert(RigidBody::Kinematic)
            // 플레이어 그래픽 숨김.
            .insert(Visibility::Hidden)
            // "방향 선택 중" 상태.
            .insert(ClockSelection::new(source))
            // 직진 도중 Clock에 진입했다면
            // 기존 직진 상태도 완전히 종료합니다.
            .remove::<StraightMovement>()
            .remove::<StraightMomentum>();
    }
}

fn advance_clock_selection(
    session: Res<PlaySession>,
    time: Res<Time<Physics>>,
    clocks: Query<&ClockBlock>,
    mut players: Query<&mut ClockSelection, With<PlayerBall>>,
) {
    if !session.is_playing() {
        return;
    }

    let delta_seconds = time.delta_secs();

    for mut selection in &mut players {
        let Ok(clock) = clocks.get(selection.source()) else {
            continue;
        };

        selection.advance(delta_seconds, *clock);
    }
}

fn launch_clock_selection_on_press(
    mut commands: Commands,
    session: Res<PlaySession>,
    keyboard: Res<ButtonInput<KeyCode>>,

    clocks: Query<(&ClockBlock, &CurrentGridPosition)>,

    mut players: Query<
        (
            Entity,
            &ClockSelection,
            &mut Position,
            &mut Transform,
            &mut LinearVelocity,
            &mut GravityScale,
            &mut Visibility,
        ),
        With<PlayerBall>,
    >,
) {
    if !session.is_playing() {
        return;
    }

    let pressed = keyboard.just_pressed(KeyCode::ArrowLeft)
        || keyboard.just_pressed(KeyCode::KeyA)
        || keyboard.just_pressed(KeyCode::ArrowRight)
        || keyboard.just_pressed(KeyCode::KeyD);

    if !pressed {
        return;
    }

    for (
        player,
        selection,
        mut position,
        mut transform,
        mut velocity,
        mut gravity_scale,
        mut visibility,
    ) in &mut players
    {
        let Ok((clock, grid_position)) = clocks.get(selection.source()) else {
            continue;
        };

        let direction = clock.launch_direction(selection.direction_index());

        let grid = grid_position.0;

        let center = Vec2::new(
            grid.x as f32 * BLOCK_WORLD_SIZE,
            grid.y as f32 * BLOCK_WORLD_SIZE,
        );

        let launch_position =
            center + direction * ClockBlock::LAUNCH_OFFSET_BLOCKS * BLOCK_WORLD_SIZE;

        position.0 = launch_position;

        transform.translation.x = launch_position.x;
        transform.translation.y = launch_position.y;

        // Clock 직진 중에는 중력 OFF.
        *gravity_scale = GravityScale(0.0);

        // 숨겨뒀던 플레이어 다시 표시.
        *visibility = Visibility::Inherited;

        // 선택 방향으로 속도 15.
        velocity.0 = direction * ClockBlock::LAUNCH_SPEED;

        commands
            .entity(player)
            // Kinematic → Dynamic.
            //
            // RigidBody는 Avian 0.7에서 immutable Component라
            // &mut Query가 아니라 insert로 교체해야 합니다.
            .insert(RigidBody::Dynamic)
            // 방금 나온 Clock Sensor에 즉시 재진입하지 않도록 잠금.
            .insert(ClockLaunchGuard::new(selection.source()))
            // Clock 선택 상태 종료.
            .remove::<ClockSelection>()
            // 이전 직진 관성이 있다면 제거.
            .remove::<StraightMomentum>()
            // Clock 발사는 입력으로 취소 불가.
            .insert(StraightMovement::press_locked(
                direction,
                ClockBlock::LAUNCH_SPEED,
            ));
    }
}

fn spawn_clock_arrow_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    clocks: Query<Entity, Added<ClockBlock>>,
) {
    let arrow_image = asset_server.load(CLOCK_ARROW_ASSET_PATH);

    for clock in &clocks {
        commands.spawn((
            Name::new("ClockArrow"),
            ClockArrowVisual { source: clock },
            Sprite {
                image: arrow_image.clone(),
                custom_size: Some(Vec2::splat(BLOCK_WORLD_SIZE)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, CLOCK_ARROW_Z),
            // Unity 원본도 선택 전에는 arrow가 비활성 상태.
            Visibility::Hidden,
            ChildOf(clock),
        ));
    }
}

fn sync_clock_arrow_visuals(
    selections: Query<&ClockSelection, With<PlayerBall>>,

    clocks: Query<(&ClockBlock, &BlockFacing)>,

    mut arrows: Query<(&ClockArrowVisual, &mut Transform, &mut Visibility)>,
) {
    let active_selection = selections.iter().next();

    for (arrow, mut transform, mut visibility) in &mut arrows {
        let Some(selection) = active_selection else {
            *visibility = Visibility::Hidden;
            continue;
        };

        // 지금 플레이어를 잡고 있는 Clock의
        // 화살표만 표시합니다.
        if selection.source() != arrow.source {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Ok((clock, facing)) = clocks.get(arrow.source) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let direction_index = selection.direction_index();

        // Unity:
        //
        // currentAngle = -arrowDir * angle
        //
        // 음수 = 시계방향.
        let world_angle = -((direction_index as f32) * clock.rotation_step_degrees()).to_radians();

        // 중요!
        //
        // Unity 원본은 arrowObj.transform.rotation,
        // 즉 "world rotation"을 직접 설정합니다.
        //
        // Bevy에서는 화살표가 Clock의 Child이므로
        // 부모 회전을 빼줘야 같은 결과가 됩니다.
        let parent_angle = (facing.0.unity_angle_degrees() as f32).to_radians();

        transform.rotation = Quat::from_rotation_z(world_angle - parent_angle);

        *visibility = Visibility::Inherited;
    }
}

fn clear_clock_launch_guard_on_exit(
    mut commands: Commands,
    mut collision_ends: MessageReader<CollisionEnd>,

    players: Query<&ClockLaunchGuard, With<PlayerBall>>,

    clocks: Query<(), With<ClockBlock>>,
) {
    for event in collision_ends.read() {
        let (player, clock) =
            if players.contains(event.collider1) && clocks.contains(event.collider2) {
                (event.collider1, event.collider2)
            } else if players.contains(event.collider2) && clocks.contains(event.collider1) {
                (event.collider2, event.collider1)
            } else {
                continue;
            };

        let Ok(guard) = players.get(player) else {
            continue;
        };

        if guard.source() != clock {
            continue;
        }

        commands.entity(player).remove::<ClockLaunchGuard>();
    }
}
