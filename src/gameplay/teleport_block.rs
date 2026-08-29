use super::{
    BLOCK_WORLD_SIZE, CurrentGridPosition, PendingPlayInteractions, PlayInteraction,
    PlayInteractionCollectSet, PlayInteractionSet, PlayerBall, ResolvedMovementInteraction,
    SolidContactResponseSet,
};
use avian2d::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeleportChannel {
    One,
    Two,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportEntrance {
    channel: TeleportChannel,
}

impl TeleportEntrance {
    pub const fn one() -> Self {
        Self {
            channel: TeleportChannel::One,
        }
    }

    pub const fn two() -> Self {
        Self {
            channel: TeleportChannel::Two,
        }
    }

    pub const fn channel(self) -> TeleportChannel {
        self.channel
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportExit {
    channel: TeleportChannel,
}

impl TeleportExit {
    pub const fn one() -> Self {
        Self {
            channel: TeleportChannel::One,
        }
    }

    pub const fn two() -> Self {
        Self {
            channel: TeleportChannel::Two,
        }
    }

    pub const fn channel(self) -> TeleportChannel {
        self.channel
    }
}

pub struct TeleportBlockPlugin;

impl Plugin for TeleportBlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PhysicsSchedule,
            collect_started_teleport_interactions
                .in_set(PlayInteractionSet::Collect)
                .in_set(PlayInteractionCollectSet::Movement),
        )
        .add_systems(
            PhysicsSchedule,
            teleport_resolved_players
                .after(SolidContactResponseSet)
                .before(PhysicsStepSystems::Sleeping),
        )
        .add_systems(Update, snap_teleported_player_visual);
    }
}

#[derive(Component, Debug, Clone, Copy)]
struct TeleportVisualSnap {
    target: Vec2,
}

fn collect_started_teleport_interactions(
    mut collision_starts: MessageReader<CollisionStart>,

    players: Query<(), With<PlayerBall>>,

    entrances: Query<&TeleportEntrance>,
    exits: Query<&TeleportExit>,

    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let (player, entrance_entity) =
            if players.contains(event.collider1) && entrances.contains(event.collider2) {
                (event.collider1, event.collider2)
            } else if players.contains(event.collider2) && entrances.contains(event.collider1) {
                (event.collider2, event.collider1)
            } else {
                continue;
            };

        let Ok(entrance) = entrances.get(entrance_entity) else {
            continue;
        };

        // Unity TeleportHole.Initialize()의 isOpen 대응:
        //
        // 같은 채널의 출구가 존재할 때만
        // 이 텔레포트 입구가 활성화됩니다.
        let has_exit = exits
            .iter()
            .any(|exit| exit.channel() == entrance.channel());

        if !has_exit {
            continue;
        }

        pending.push(PlayInteraction::movement(entrance_entity, player));
    }
}

fn teleport_resolved_players(
    mut resolved_movements: MessageReader<ResolvedMovementInteraction>,
    entrances: Query<&TeleportEntrance>,
    exits: Query<(&TeleportExit, &CurrentGridPosition)>,
    mut players: Query<(&mut Position, &mut Transform), With<PlayerBall>>,
    mut commands: Commands,
) {
    for movement in resolved_movements.read() {
        let source = movement.source();
        let player = movement.actor();

        let Ok(entrance) = entrances.get(source) else {
            // Clock이나 다른 Movement 블록에서 발생한 이벤트는
            // 이 시스템이 처리하지 않습니다.
            continue;
        };

        let Some((_, exit_position)) = exits
            .iter()
            .find(|(exit, _)| exit.channel() == entrance.channel())
        else {
            continue;
        };

        let Ok((mut position, mut transform)) = players.get_mut(player) else {
            continue;
        };

        let grid = exit_position.0;

        let target = Vec2::new(
            grid.x as f32 * BLOCK_WORLD_SIZE,
            grid.y as f32 * BLOCK_WORLD_SIZE,
        );

        // Unity TeleportHole.OnTriggerEnter2D()와 동일하게
        // 위치만 변경합니다.
        //
        // LinearVelocity
        // GravityScale
        // StraightMovement
        // StraightMomentum
        //
        // 등은 일부러 건드리지 않습니다.
        position.0 = target;

        transform.translation.x = target.x;
        transform.translation.y = target.y;

        commands
            .entity(player)
            .insert(TeleportVisualSnap { target });
    }
}

fn snap_teleported_player_visual(
    mut commands: Commands,
    mut players: Query<(Entity, &TeleportVisualSnap, &mut Transform), With<PlayerBall>>,
) {
    for (player, snap, mut transform) in &mut players {
        // RunFixedMainLoop의 interpolation 이후
        // Update에서 다시 출구 위치로 정확히 snap합니다.
        //
        // z 값은 건드리지 않습니다.
        transform.translation = snap.target.extend(transform.translation.z);

        commands.entity(player).remove::<TeleportVisualSnap>();
    }
}
