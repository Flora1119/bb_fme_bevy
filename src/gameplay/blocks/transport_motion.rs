use super::{StraightTransportBlock, StraightTransportMode};
use crate::{
    domain::{CardinalDirection, GridPosition, GridSize},
    gameplay::{
        ActivePlayWorld, BLOCK_WORLD_SIZE, BlockFacing, CurrentGridPosition, GridIndex,
        MapSpawnSet, PhysicsInitializationSet, PlayInteractionSet, PlaySession, PlaySessionSet,
        PlayWorld, ResolvedMovementInteraction, SolidContactResponseSet, TeleportResolutionSet,
    },
};
use avian2d::prelude::{Physics, PhysicsSchedule, PhysicsStepSystems, Position};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct StraightTransportPlugin;

impl Plugin for StraightTransportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingTransportSteps>()
            .add_systems(
                Update,
                initialize_transport_runtime
                    .after(MapSpawnSet)
                    .after(PhysicsInitializationSet),
            )
            .add_systems(
                PhysicsSchedule,
                animate_straight_transports
                    .after(PhysicsStepSystems::First)
                    .after(PlaySessionSet::AdvanceTime)
                    .before(PhysicsStepSystems::BroadPhase),
            )
            .add_systems(
                PhysicsSchedule,
                begin_transport_steps
                    .after(animate_straight_transports)
                    .after(PlayInteractionSet::Resolve)
                    .after(SolidContactResponseSet)
                    .after(TeleportResolutionSet)
                    .before(PhysicsStepSystems::Sleeping),
            );
    }
}

/// 이번 물리 틱에서 새롭게 밟힌 이동 블록입니다.
#[derive(Resource, Default)]
pub struct PendingTransportSteps(HashSet<Entity>);

impl PendingTransportSteps {
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn push(&mut self, entity: Entity) {
        self.0.insert(entity);
    }
}

#[derive(Component, Default)]
struct TransportRuntime {
    motion: Option<TransportMotion>,
    remaining_steps: u32,
    wait_seconds: f32,
}

#[derive(Clone, Copy)]
struct TransportMotion {
    start: Vec2,
    target: Vec2,
    elapsed_seconds: f32,
}

fn initialize_transport_runtime(
    mut commands: Commands,
    blocks: Query<Entity, (With<StraightTransportBlock>, Without<TransportRuntime>)>,
) {
    for entity in &blocks {
        commands.entity(entity).insert(TransportRuntime::default());
    }
}

fn transport_world_position(grid: GridPosition) -> Vec2 {
    Vec2::new(
        grid.x as f32 * BLOCK_WORLD_SIZE,
        grid.y as f32 * BLOCK_WORLD_SIZE,
    )
}

fn next_transport_cell(
    current: GridPosition,
    direction: CardinalDirection,
    size: GridSize,
) -> Option<GridPosition> {
    let (dx, dy) = direction.unit_offset();

    let next = GridPosition::new(current.x.checked_add(dx)?, current.y.checked_add(dy)?);

    // Unity TransportBlock.IsPositionInMap과 동일하게
    // 맵 크기의 끝값을 포함합니다.
    let inside = next.x >= 0 && next.x <= size.width && next.y >= 0 && next.y <= size.height;

    inside.then_some(next)
}

fn animate_straight_transports(
    time: Res<Time<Physics>>,
    session: Res<PlaySession>,
    mut blocks: Query<(&mut TransportRuntime, &mut Position, &mut Transform)>,
) {
    if !session.is_playing() {
        return;
    }

    let dt = time.delta_secs();

    for (mut runtime, mut position, mut transform) in &mut blocks {
        runtime.wait_seconds = (runtime.wait_seconds - dt).max(0.0);

        let Some(mut motion) = runtime.motion.take() else {
            continue;
        };

        motion.elapsed_seconds += dt;

        let progress = (motion.elapsed_seconds / StraightTransportBlock::MOVE_DURATION_SECONDS)
            .clamp(0.0, 1.0);

        let next = if progress >= 1.0 {
            motion.target
        } else {
            // Unity와 같은 EaseOutSine 보간입니다.
            let eased = (progress * std::f32::consts::FRAC_PI_2).sin();

            motion.start.lerp(motion.target, eased)
        };

        position.0 = next;
        transform.translation.x = next.x;
        transform.translation.y = next.y;

        if progress < 1.0 {
            runtime.motion = Some(motion);
        }
    }
}

fn begin_transport_steps(
    session: Res<PlaySession>,
    active_world: Res<ActivePlayWorld>,
    worlds: Query<&PlayWorld>,
    mut pending: ResMut<PendingTransportSteps>,
    mut resolved_movements: MessageReader<ResolvedMovementInteraction>,
    mut index: ResMut<GridIndex>,
    mut blocks: Query<(
        Entity,
        &StraightTransportBlock,
        &BlockFacing,
        &mut CurrentGridPosition,
        &mut TransportRuntime,
        &Position,
    )>,
) {
    // 매 틱 소비하여 이전 접촉 요청이 나중에 재발동하지 않게 합니다.
    let stepped = std::mem::take(&mut pending.0);

    // 포탈/텔레포트로 이동했다면 이전 위치의 밟기 요청은 무효입니다.
    // 이미 진행 중이던 연속 이동은 계속 처리합니다.
    let player_was_relocated = resolved_movements.read().count() > 0;

    if !session.is_playing() {
        return;
    }

    let Some(root) = active_world.root() else {
        return;
    };

    let Ok(play_world) = worlds.get(root) else {
        return;
    };

    let size = play_world.definition().settings.size;

    // 여러 블록이 같은 목적지를 노려도 처리 순서를 일정하게 합니다.
    let mut entities: Vec<Entity> = blocks.iter().map(|(entity, ..)| entity).collect();

    entities.sort_unstable_by_key(|entity| entity.to_bits());

    for entity in entities {
        let Ok((_, block, facing, mut current, mut runtime, position)) = blocks.get_mut(entity)
        else {
            continue;
        };

        if !player_was_relocated && stepped.contains(&entity) {
            // 다시 밟으면 Unity처럼 연속 이동을 새로 시작합니다.
            runtime.remaining_steps = match block.mode() {
                StraightTransportMode::OneStep => 1,
                StraightTransportMode::Continuous => StraightTransportBlock::MAX_CONTINUOUS_STEPS,
            };

            runtime.wait_seconds = 0.0;
        }

        if runtime.remaining_steps == 0 || runtime.wait_seconds > 0.000_001 {
            continue;
        }

        let Some(destination) = next_transport_cell(current.0, facing.0, size) else {
            runtime.remaining_steps = 0;
            continue;
        };

        if !index.try_relocate(entity, &mut current, destination) {
            // 막히면 종료합니다. 길이 열려도 자동 재출발하지 않습니다.
            runtime.remaining_steps = 0;
            continue;
        }

        runtime.motion = Some(TransportMotion {
            // 이동 중 다시 발동해도 현재 실제 위치에서 이어갑니다.
            start: position.0,
            target: transport_world_position(destination),
            elapsed_seconds: 0.0,
        });

        runtime.remaining_steps -= 1;
        runtime.wait_seconds = StraightTransportBlock::STEP_INTERVAL_SECONDS;
    }
}

#[cfg(test)]
mod transport_motion_tests {
    use super::*;
    use crate::{
        block::{BlockAssetConfig, BlockCategory},
        domain::ValidatedMap,
        gameplay::{MapSpawnPlugin, PlaySessionPlugin, SpawnValidatedMap},
        map::MapDocument,
    };
    use std::time::Duration;

    fn test_app(block_id: &str) -> (App, Entity) {
        let config: BlockAssetConfig = serde_json::from_str(include_str!(
            "../../../assets/config/block_assets_config.json"
        ))
        .unwrap();

        let document: MapDocument = serde_json::from_str(include_str!(
            "../../../assets/maps/synthetic_minimal_map.json"
        ))
        .unwrap();

        let mut map = ValidatedMap::from_document(&document, &config).unwrap();

        let block = map
            .blocks
            .iter_mut()
            .find(|block| block.id.as_str() == "b_normal")
            .unwrap();

        block.id = block_id.into();
        block.category = BlockCategory::Transport;
        block.direction = CardinalDirection::Right;

        let mut app = App::new();

        app.init_resource::<Time<Physics>>().add_plugins((
            MapSpawnPlugin,
            PlaySessionPlugin,
            StraightTransportPlugin,
        ));

        app.world_mut().write_message(SpawnValidatedMap(map));
        app.update();

        let entity = app
            .world()
            .resource::<GridIndex>()
            .entity_at(GridPosition::new(2, 0))
            .unwrap();

        // 이 테스트는 물리 엔진 대신 이동 시스템만 실행합니다.
        app.world_mut()
            .entity_mut(entity)
            .insert(Position(Vec2::new(2.0, 0.0)));

        (app, entity)
    }

    fn tick(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time<Physics>>()
            .advance_by(Duration::from_millis(20));

        app.world_mut().run_schedule(PhysicsSchedule);
    }

    fn step_on(app: &mut App, entity: Entity) {
        app.world_mut()
            .resource_mut::<PendingTransportSteps>()
            .push(entity);

        tick(app);
    }

    fn grid(app: &App, entity: Entity) -> GridPosition {
        app.world().get::<CurrentGridPosition>(entity).unwrap().0
    }

    #[test]
    fn one_step_reserves_destination_then_animates_once() {
        let (mut app, entity) = test_app("tp_st_o");

        step_on(&mut app, entity);

        assert_eq!(grid(&app, entity), GridPosition::new(3, 0));

        let index = app.world().resource::<GridIndex>();
        assert_eq!(index.entity_at(GridPosition::new(2, 0)), None);
        assert_eq!(index.entity_at(GridPosition::new(3, 0)), Some(entity),);

        // 논리 좌표가 먼저 바뀌고 실제 위치는 다음 틱부터 이동합니다.
        assert_eq!(
            app.world().get::<Position>(entity).unwrap().0,
            Vec2::new(2.0, 0.0),
        );

        tick(&mut app);

        let x = app.world().get::<Position>(entity).unwrap().x;
        assert!(x > 2.0 && x < 3.0);

        for _ in 0..50 {
            tick(&mut app);
        }

        assert_eq!(grid(&app, entity), GridPosition::new(3, 0));
        assert_eq!(
            app.world().get::<Position>(entity).unwrap().0,
            Vec2::new(3.0, 0.0),
        );

        let transform = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.truncate(), Vec2::new(3.0, 0.0));
    }

    #[test]
    fn continuous_transport_waits_half_a_second_between_steps() {
        let (mut app, entity) = test_app("tp_st_a");

        step_on(&mut app, entity);
        assert_eq!(grid(&app, entity), GridPosition::new(3, 0));

        for _ in 0..24 {
            tick(&mut app);
        }

        assert_eq!(grid(&app, entity), GridPosition::new(3, 0));

        tick(&mut app);

        assert_eq!(grid(&app, entity), GridPosition::new(4, 0));

        // 기존 테스트 맵의 (12, 0)에 가시가 있으므로 그 앞에서 멈춥니다.
        for _ in 0..300 {
            tick(&mut app);
        }

        assert_eq!(grid(&app, entity), GridPosition::new(11, 0));
        assert_eq!(
            app.world().get::<Position>(entity).unwrap().0,
            Vec2::new(11.0, 0.0),
        );
    }

    #[test]
    fn dead_session_discards_new_step_requests() {
        let (mut app, entity) = test_app("tp_st_o");

        app.world_mut().resource_mut::<PlaySession>().mark_dead();

        step_on(&mut app, entity);

        assert_eq!(grid(&app, entity), GridPosition::new(2, 0));

        app.world_mut().resource_mut::<PlaySession>().reset();

        tick(&mut app);

        // 사망 중 받은 요청이 상태 복구 뒤에 되살아나면 안 됩니다.
        assert_eq!(grid(&app, entity), GridPosition::new(2, 0));
    }

    #[test]
    fn transport_bounds_preserve_unity_inclusive_endpoints() {
        let size = GridSize {
            width: 4,
            height: 3,
        };

        assert_eq!(
            next_transport_cell(GridPosition::new(3, 2), CardinalDirection::Right, size,),
            Some(GridPosition::new(4, 2)),
        );

        assert_eq!(
            next_transport_cell(GridPosition::new(4, 2), CardinalDirection::Right, size,),
            None,
        );

        assert_eq!(
            next_transport_cell(GridPosition::new(2, 0), CardinalDirection::Down, size,),
            None,
        );
    }
}
