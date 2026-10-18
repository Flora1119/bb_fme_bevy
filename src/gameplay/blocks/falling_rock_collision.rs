use super::FallingRock;
use crate::gameplay::{
    BlockIdentity, MapBoundary, PendingPlayInteractions, PlayInteraction,
    PlayInteractionCollectSet, PlayInteractionSet, PlaySession, PlayerBall,
};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::HashSet;

pub struct FallingRockCollisionPlugin;

impl Plugin for FallingRockCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingFallingRockRemovals>()
            .add_systems(
                PhysicsSchedule,
                collect_falling_rock_contacts
                    .in_set(PlayInteractionSet::Collect)
                    .in_set(PlayInteractionCollectSet::Death),
            )
            .add_systems(
                PhysicsSchedule,
                remove_contacted_falling_rocks
                    .after(PlayInteractionSet::Resolve)
                    .before(PhysicsStepSystems::Sleeping),
            );
    }
}

#[derive(Resource, Default)]
struct PendingFallingRockRemovals(HashSet<Entity>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FallingRockContact {
    Player,
    Block,
    Boundary,
    Other,
}

fn collect_falling_rock_contacts(
    mut collision_starts: MessageReader<CollisionStart>,
    session: Res<PlaySession>,
    rocks: Query<&FallingRock>,
    players: Query<(), With<PlayerBall>>,
    blocks: Query<(), (With<BlockIdentity>, Without<PlayerBall>)>,
    boundaries: Query<(), With<MapBoundary>>,
    parents: Query<&ChildOf>,
    mut pending: ResMut<PendingPlayInteractions>,
    mut removals: ResMut<PendingFallingRockRemovals>,
) {
    removals.0.clear();

    for event in collision_starts.read() {
        // 플레이 종료 중에도 메시지는 소비합니다.
        if !session.is_playing() {
            continue;
        }

        let (rock_entity, rock, other) = if let Ok(rock) = rocks.get(event.collider1) {
            (event.collider1, rock, event.collider2)
        } else if let Ok(rock) = rocks.get(event.collider2) {
            (event.collider2, rock, event.collider1)
        } else {
            continue;
        };

        let mut current = other;

        let contact = loop {
            if players.contains(current) {
                break FallingRockContact::Player;
            }

            if boundaries.contains(current) {
                break FallingRockContact::Boundary;
            }

            if blocks.contains(current) {
                break FallingRockContact::Block;
            }

            // 반쪽 블록이나 복합 가시처럼 충돌체가 자식에 있는 경우,
            // 부모를 따라 올라가 실제 맵 오브젝트를 확인합니다.
            let Ok(parent) = parents.get(current) else {
                break FallingRockContact::Other;
            };

            current = parent.parent();
        };

        if contact == FallingRockContact::Player {
            // 원본 Damage 태그는 페이드인·대기 중에도 유효합니다.
            pending.push(PlayInteraction::death(rock_entity));
        }

        if rock.is_falling()
            && matches!(
                contact,
                FallingRockContact::Player
                    | FallingRockContact::Block
                    | FallingRockContact::Boundary
            )
        {
            // 같은 틱에 여러 충돌이 있어도 제거 요청은 하나로 합칩니다.
            removals.0.insert(rock_entity);
        }
    }
}

fn remove_contacted_falling_rocks(
    mut commands: Commands,
    mut removals: ResMut<PendingFallingRockRemovals>,
    rocks: Query<(), With<FallingRock>>,
) {
    let mut entities: Vec<Entity> = removals.0.drain().collect();
    entities.sort_unstable_by_key(|entity| entity.to_bits());

    for entity in entities {
        if rocks.contains(entity) {
            commands.entity(entity).despawn();
        }
    }
}
