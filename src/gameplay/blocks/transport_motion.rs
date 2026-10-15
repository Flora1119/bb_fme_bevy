use super::{PushPullBlock, StraightTransportBlock, StraightTransportMode, plan_push_pull};
use crate::gameplay::RuntimeBlock;
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

/// 이번 물리 틱에서 새롭게 밟힌 이동 또는 밀기·당기기 블록입니다.
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
    blocks: Query<Entity, (With<RuntimeBlock>, Without<TransportRuntime>)>,
) {
    // 밀기·당기기의 대상도 동일한 이동 상태를 사용합니다.
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
    mut blocks: Query<(&mut TransportRuntime, Option<&mut Position>, &mut Transform)>,
) {
    if !session.is_playing() {
        return;
    }

    let dt = time.delta_secs();

    for (mut runtime, position, mut transform) in &mut blocks {
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
            let eased = (progress * std::f32::consts::FRAC_PI_2).sin();

            motion.start.lerp(motion.target, eased)
        };

        // 물리 바디가 있는 대상은 물리 위치도 함께 갱신합니다.
        if let Some(mut position) = position {
            position.0 = next;
        }

        // 포탈 출구처럼 Position이 없는 대상도 이동할 수 있습니다.
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
    mut blocks: Query<
        (
            Entity,
            Option<&StraightTransportBlock>,
            Option<&PushPullBlock>,
            &BlockFacing,
            &mut CurrentGridPosition,
            &mut TransportRuntime,
            Option<&Position>,
            &Transform,
        ),
        With<RuntimeBlock>,
    >,
) {
    // 처리할 수 없는 상태여도 요청은 소비합니다.
    let stepped = std::mem::take(&mut pending.0);
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

    // 이동 블록과 밀기·당기기를 같은 순서 안에서 처리합니다.
    // 앞선 이동의 점유 변경은 다음 계산에 즉시 반영됩니다.
    let mut entities: Vec<Entity> = blocks
        .iter()
        .filter_map(|(entity, straight, push_pull, ..)| {
            (straight.is_some() || push_pull.is_some()).then_some(entity)
        })
        .collect();

    entities.sort_unstable_by_key(|entity| entity.to_bits());

    for entity in entities {
        let Ok((_, straight, push_pull, facing, current, ..)) = blocks.get(entity) else {
            continue;
        };

        // 대상 블록을 수정하기 전에 발동 블록의 정보만 복사합니다.
        let straight = straight.copied();
        let push_pull = push_pull.copied();
        let direction = facing.0;
        let actor_grid = current.0;
        let newly_stepped = !player_was_relocated && stepped.contains(&entity);

        if let Some(block) = push_pull {
            if !newly_stepped {
                continue;
            }

            let Some(plan) = plan_push_pull(block, actor_grid, direction, size, |position| {
                index.entity_at(position)
            }) else {
                continue;
            };

            let Ok((_, _, _, _, mut target_grid, mut runtime, position, transform)) =
                blocks.get_mut(plan.target)
            else {
                continue;
            };

            // 계획의 출발점과 대상의 현재 위치가 일치하는지 확인합니다.
            if target_grid.0 != plan.source {
                continue;
            }

            if !index.try_relocate(plan.target, &mut target_grid, plan.destination) {
                continue;
            }

            let start = position.map_or(transform.translation.truncate(), |position| position.0);

            // 이전 이동을 교체하고, 현재 실제 위치에서 이동을 시작합니다.
            runtime.motion = Some(TransportMotion {
                start,
                target: transport_world_position(plan.destination),
                elapsed_seconds: 0.0,
            });

            // 대상이 연속 이동 블록이어도 남은 횟수와 대기 시간은 유지합니다.
            continue;
        }

        let Some(block) = straight else {
            continue;
        };

        let Ok((_, _, _, _, mut current, mut runtime, position, transform)) =
            blocks.get_mut(entity)
        else {
            continue;
        };

        if newly_stepped {
            runtime.remaining_steps = match block.mode() {
                StraightTransportMode::OneStep => 1,
                StraightTransportMode::Continuous => StraightTransportBlock::MAX_CONTINUOUS_STEPS,
            };

            runtime.wait_seconds = 0.0;
        }

        if runtime.remaining_steps == 0 || runtime.wait_seconds > 0.000_001 {
            continue;
        }

        let Some(destination) = next_transport_cell(current.0, direction, size) else {
            runtime.remaining_steps = 0;
            continue;
        };

        if !index.try_relocate(entity, &mut current, destination) {
            runtime.remaining_steps = 0;
            continue;
        }

        let start = position.map_or(transform.translation.truncate(), |position| position.0);

        runtime.motion = Some(TransportMotion {
            start,
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

    fn push_pull_target(app: &App) -> Entity {
        // 기존 최소 테스트 맵에 있는 가시를 이동 대상으로 사용합니다.
        app.world()
            .resource::<GridIndex>()
            .entity_at(GridPosition::new(12, 0))
            .unwrap()
    }

    #[test]
    fn push_pull_updates_grid_then_animates_all_four_variants() {
        for (block_id, destination_x) in [
            ("tp_push_o", 13),
            ("tp_push_a", 25),
            ("tp_pull_o", 11),
            ("tp_pull_a", 3),
        ] {
            let (mut app, owner) = test_app(block_id);
            let target = push_pull_target(&app);
            let source = GridPosition::new(12, 0);
            let destination = GridPosition::new(destination_x, 0);

            app.world_mut()
                .entity_mut(target)
                .insert(Position(Vec2::new(12.0, 0.0)));

            step_on(&mut app, owner);

            assert_eq!(grid(&app, owner), GridPosition::new(2, 0));
            assert_eq!(grid(&app, target), destination);

            let index = app.world().resource::<GridIndex>();
            assert_eq!(index.entity_at(source), None);
            assert_eq!(index.entity_at(destination), Some(target));

            // 논리 좌표만 먼저 변경됩니다.
            assert_eq!(
                app.world().get::<Position>(target).unwrap().0,
                Vec2::new(12.0, 0.0),
            );

            for _ in 0..20 {
                tick(&mut app);
            }

            let expected = Vec2::new(destination_x as f32, 0.0);

            assert_eq!(app.world().get::<Position>(target).unwrap().0, expected,);
            assert_eq!(
                app.world()
                    .get::<Transform>(target)
                    .unwrap()
                    .translation
                    .truncate(),
                expected,
            );
            assert_eq!(
                app.world()
                    .get::<crate::gameplay::OriginGridPosition>(target)
                    .unwrap()
                    .0,
                source,
            );
        }
    }

    #[test]
    fn push_pull_moves_targets_without_physics_position() {
        let (mut app, owner) = test_app("tp_push_o");
        let target = push_pull_target(&app);

        assert!(app.world().get::<Position>(target).is_none());

        step_on(&mut app, owner);

        for _ in 0..20 {
            tick(&mut app);
        }

        assert_eq!(grid(&app, target), GridPosition::new(13, 0));
        assert_eq!(
            app.world()
                .get::<Transform>(target)
                .unwrap()
                .translation
                .truncate(),
            Vec2::new(13.0, 0.0),
        );
        assert!(app.world().get::<Position>(target).is_none());
    }

    #[test]
    fn push_pull_retargets_an_active_motion_without_returning_to_old_destination() {
        let (mut app, owner) = test_app("tp_push_o");
        let target = push_pull_target(&app);

        app.world_mut().entity_mut(target).insert((
            StraightTransportBlock::continuous(),
            BlockFacing(CardinalDirection::Right),
            Position(Vec2::new(12.0, 0.0)),
        ));

        // 대상이 스스로 12 -> 13 이동을 시작합니다.
        step_on(&mut app, target);

        for _ in 0..5 {
            tick(&mut app);
        }

        let before_push = app.world().get::<Position>(target).unwrap().x;
        assert!(before_push > 12.0 && before_push < 13.0);

        // 이동 중인 대상의 현재 논리 칸 13을 찾아 14로 밉니다.
        step_on(&mut app, owner);

        assert_eq!(grid(&app, target), GridPosition::new(14, 0));

        let runtime = app.world().get::<TransportRuntime>(target).unwrap();
        assert_eq!(
            runtime.remaining_steps,
            StraightTransportBlock::MAX_CONTINUOUS_STEPS - 1,
        );

        // 다음 자체 이동 시점인 0.5초에 도달하기 전에 확인합니다.
        for _ in 0..16 {
            tick(&mut app);
        }

        assert_eq!(grid(&app, target), GridPosition::new(14, 0));
        assert_eq!(
            app.world().get::<Position>(target).unwrap().0,
            Vec2::new(14.0, 0.0),
        );
    }

    #[test]
    fn push_pull_discards_requests_when_player_was_relocated() {
        let (mut app, owner) = test_app("tp_push_o");
        let target = push_pull_target(&app);

        app.world_mut()
            .write_message(ResolvedMovementInteraction::new(owner, owner));

        step_on(&mut app, owner);
        tick(&mut app);

        assert_eq!(grid(&app, target), GridPosition::new(12, 0));
    }
}
