use super::{PushPullAction, PushPullBlock, PushPullReach};
use crate::domain::{CardinalDirection, GridPosition, GridSize};
use bevy::prelude::Entity;

/// 대상 탐색과 밀기 목적지 탐색의 최대 검사 칸 수입니다.
const MAX_SCAN_CELLS: usize = 100;

/// 아직 적용하지 않은 이동 계획입니다.
///
/// 실제 적용 시에는 GridIndex::try_relocate()로
/// 대상의 현재 위치와 목적지 점유 상태를 다시 확인합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushPullPlan {
    pub target: Entity,
    pub source: GridPosition,
    pub destination: GridPosition,
}

fn inside_map(position: GridPosition, size: GridSize) -> bool {
    // 기존 이동 블록처럼 맵 크기의 끝값을 포함합니다.
    position.x >= 0 && position.x <= size.width && position.y >= 0 && position.y <= size.height
}

fn next_cell(
    position: GridPosition,
    direction: CardinalDirection,
    size: GridSize,
) -> Option<GridPosition> {
    let (dx, dy) = direction.unit_offset();

    let next = GridPosition::new(position.x.checked_add(dx)?, position.y.checked_add(dy)?);

    inside_map(next, size).then_some(next)
}

/// 현재 격자의 점유 정보를 읽어 이동 계획을 계산합니다.
///
/// entity_at에는 |position| index.entity_at(position)을 전달합니다.
/// 이 함수는 격자, 컴포넌트, 실제 위치를 변경하지 않습니다.
pub fn plan_push_pull(
    block: PushPullBlock,
    current: GridPosition,
    direction: CardinalDirection,
    size: GridSize,
    entity_at: impl Fn(GridPosition) -> Option<Entity>,
) -> Option<PushPullPlan> {
    if !inside_map(current, size) {
        return None;
    }

    // 자신의 바로 앞부터 탐색하여 처음 발견한 블록만 선택합니다.
    let mut cursor = current;
    let mut found = None;

    for _ in 0..MAX_SCAN_CELLS {
        let Some(next) = next_cell(cursor, direction, size) else {
            break;
        };

        cursor = next;

        if let Some(target) = entity_at(cursor) {
            found = Some((target, cursor));
            break;
        }
    }

    let (target, source) = found?;

    let destination = match (block.action(), block.reach()) {
        (PushPullAction::Pull, PushPullReach::OneCell) => {
            next_cell(source, direction.opposite(), size)?
        }

        (PushPullAction::Pull, PushPullReach::ToLimit) => {
            // 最初に見つけた対象なので、自分との間は空いています。
            next_cell(current, direction, size)?
        }

        (PushPullAction::Push, PushPullReach::OneCell) => next_cell(source, direction, size)?,

        (PushPullAction::Push, PushPullReach::ToLimit) => {
            let mut destination = source;

            for _ in 0..MAX_SCAN_CELLS {
                let Some(next) = next_cell(destination, direction, size) else {
                    break;
                };

                if entity_at(next).is_some() {
                    break;
                }

                destination = next;
            }

            destination
        }
    };

    // 隣接した引き寄せ対象など、移動しない場合は計画を作りません。
    if destination == source || destination == current {
        return None;
    }

    if entity_at(destination).is_some() {
        return None;
    }

    Some(PushPullPlan {
        target,
        source,
        destination,
    })
}

#[cfg(test)]
mod push_pull_plan_tests {
    use super::*;
    use bevy::prelude::World;
    use std::collections::HashMap;

    fn offset(origin: GridPosition, direction: CardinalDirection, distance: i32) -> GridPosition {
        let (dx, dy) = direction.unit_offset();

        GridPosition::new(origin.x + dx * distance, origin.y + dy * distance)
    }

    fn entities() -> (Entity, Entity, Entity) {
        let mut world = World::new();

        (
            world.spawn_empty().id(),
            world.spawn_empty().id(),
            world.spawn_empty().id(),
        )
    }

    #[test]
    fn push_pull_plan_handles_all_variants_in_all_directions() {
        let current = GridPosition::new(10, 10);
        let size = GridSize {
            width: 20,
            height: 20,
        };

        let cases = [
            (PushPullAction::Push, PushPullReach::OneCell, 4),
            (PushPullAction::Push, PushPullReach::ToLimit, 5),
            (PushPullAction::Pull, PushPullReach::OneCell, 2),
            (PushPullAction::Pull, PushPullReach::ToLimit, 1),
        ];

        for direction in CardinalDirection::ALL {
            for (action, reach, expected_distance) in cases {
                let (owner, target, blocker) = entities();
                let source = offset(current, direction, 3);

                let cells = HashMap::from([
                    (current, owner),
                    (source, target),
                    (offset(current, direction, 6), blocker),
                ]);

                let plan = plan_push_pull(
                    PushPullBlock::new(action, reach),
                    current,
                    direction,
                    size,
                    |position| cells.get(&position).copied(),
                )
                .unwrap();

                assert_eq!(plan.target, target);
                assert_eq!(plan.source, source);
                assert_eq!(
                    plan.destination,
                    offset(current, direction, expected_distance),
                );

                // 計画を作るだけなので、対象は元のセルに残ります。
                assert_eq!(cells.get(&source), Some(&target));
                assert_eq!(cells.get(&plan.destination), None);
            }
        }
    }

    #[test]
    fn push_pull_plan_does_not_skip_a_blocked_first_target() {
        let current = GridPosition::new(1, 1);
        let source = GridPosition::new(3, 1);
        let (owner, target, blocker) = entities();

        let cells = HashMap::from([
            (current, owner),
            (source, target),
            (GridPosition::new(4, 1), blocker),
        ]);

        for reach in [PushPullReach::OneCell, PushPullReach::ToLimit] {
            let plan = plan_push_pull(
                PushPullBlock::new(PushPullAction::Push, reach),
                current,
                CardinalDirection::Right,
                GridSize {
                    width: 10,
                    height: 10,
                },
                |position| cells.get(&position).copied(),
            );

            assert_eq!(plan, None);
        }
    }

    #[test]
    fn push_pull_plan_does_not_pull_an_adjacent_target_into_owner() {
        let current = GridPosition::new(2, 2);
        let size = GridSize {
            width: 4,
            height: 4,
        };

        for direction in CardinalDirection::ALL {
            let (owner, target, _) = entities();

            let cells = HashMap::from([(current, owner), (offset(current, direction, 1), target)]);

            for reach in [PushPullReach::OneCell, PushPullReach::ToLimit] {
                assert_eq!(
                    plan_push_pull(
                        PushPullBlock::new(PushPullAction::Pull, reach),
                        current,
                        direction,
                        size,
                        |position| cells.get(&position).copied(),
                    ),
                    None,
                );
            }
        }
    }

    #[test]
    fn push_pull_plan_stops_at_inclusive_map_edges() {
        let current = GridPosition::new(2, 2);
        let size = GridSize {
            width: 4,
            height: 4,
        };

        for direction in CardinalDirection::ALL {
            let (owner, target, _) = entities();
            let edge = offset(current, direction, 2);

            let cells = HashMap::from([(current, owner), (offset(current, direction, 1), target)]);

            let plan = plan_push_pull(
                PushPullBlock::new(PushPullAction::Push, PushPullReach::ToLimit),
                current,
                direction,
                size,
                |position| cells.get(&position).copied(),
            )
            .unwrap();

            assert_eq!(plan.destination, edge);

            // 境界上の対象は、さらに外側へ押せません。
            let cells = HashMap::from([(current, owner), (edge, target)]);

            for reach in [PushPullReach::OneCell, PushPullReach::ToLimit] {
                assert_eq!(
                    plan_push_pull(
                        PushPullBlock::new(PushPullAction::Push, reach),
                        current,
                        direction,
                        size,
                        |position| cells.get(&position).copied(),
                    ),
                    None,
                );
            }
        }
    }

    #[test]
    fn push_pull_plan_returns_none_when_no_target_exists() {
        assert_eq!(
            plan_push_pull(
                PushPullBlock::new(PushPullAction::Pull, PushPullReach::ToLimit),
                GridPosition::new(2, 2),
                CardinalDirection::Right,
                GridSize {
                    width: 10,
                    height: 10,
                },
                |_| None,
            ),
            None,
        );
    }

    #[test]
    fn push_pull_plan_limits_target_search_to_one_hundred_cells() {
        let current = GridPosition::new(0, 0);
        let (_, target, _) = entities();
        let size = GridSize {
            width: 200,
            height: 1,
        };

        for (distance, expected) in [(100, true), (101, false)] {
            let source = GridPosition::new(distance, 0);

            let plan = plan_push_pull(
                PushPullBlock::new(PushPullAction::Pull, PushPullReach::OneCell),
                current,
                CardinalDirection::Right,
                size,
                |position| (position == source).then_some(target),
            );

            assert_eq!(plan.is_some(), expected);

            if let Some(plan) = plan {
                assert_eq!(plan.destination, GridPosition::new(distance - 1, 0),);
            }
        }
    }
}
