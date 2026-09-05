use super::{
    AbilityInventory, AbilityUseIntent, PlayerAbility,
    dash::{PlayerDashState, try_start_dash},
    gravity::{PlayerGravityState, invert_player_gravity},
    jump::jump_ability_velocity,
    straight::{ITEM_STRAIGHT_SPEED, straight_ability_direction, straight_ability_velocity},
    teleport::{TeleportCheckpoint, teleport_player_to_checkpoint},
};
use crate::gameplay::{PlaySession, PlayerBall, StraightMomentum, StraightMovement};
use avian2d::prelude::{Gravity, GravityScale, LinearVelocity, Position};
use bevy::prelude::*;

pub(super) fn apply_queued_ability_use(
    mut commands: Commands,
    session: Res<PlaySession>,
    checkpoint: Res<TeleportCheckpoint>,
    mut world_gravity: ResMut<Gravity>,
    mut gravity_state: ResMut<PlayerGravityState>,
    mut inventory: ResMut<AbilityInventory>,
    mut players: Query<
        (
            Entity,
            &mut AbilityUseIntent,
            &mut LinearVelocity,
            Option<&mut GravityScale>,
            &mut PlayerDashState,
            &mut Position,
            &mut Transform,
        ),
        With<PlayerBall>,
    >,
) {
    if !session.is_playing() {
        for (_, mut intent, _, _, _, _, _) in &mut players {
            intent.clear();
        }

        return;
    }

    for (
        player,
        mut intent,
        mut velocity,
        gravity_scale,
        mut dash_state,
        mut position,
        mut transform,
    ) in &mut players
    {
        let Some(direction) = intent.take() else {
            continue;
        };

        match inventory.current() {
            Some(PlayerAbility::Jump) => {
                velocity.0 = jump_ability_velocity(velocity.0);

                let consumed = inventory.pop_current();

                debug_assert_eq!(consumed, Some(PlayerAbility::Jump));
            }

            Some(PlayerAbility::Dash) => {
                try_start_dash(&mut dash_state, &mut velocity, direction);

                // Unity 원본 호환:
                // 쿨타임으로 실제 Dash가 발동되지 않아도
                // 현재 i_dash는 소비됩니다.
                let consumed = inventory.pop_current();

                debug_assert_eq!(consumed, Some(PlayerAbility::Dash));
            }

            Some(PlayerAbility::Straight) => {
                let Some(mut gravity_scale) = gravity_scale else {
                    continue;
                };

                let direction_vector = straight_ability_direction(direction);

                velocity.0 = straight_ability_velocity(direction);

                *gravity_scale = GravityScale(0.0);

                // 새 직진이 이전 직진 관성을 완전히 대체합니다.
                commands
                    .entity(player)
                    .remove::<StraightMomentum>()
                    .insert(StraightMovement::new(direction_vector, ITEM_STRAIGHT_SPEED));

                let consumed = inventory.pop_current();

                debug_assert_eq!(consumed, Some(PlayerAbility::Straight));
            }

            Some(PlayerAbility::Teleport) => {
                teleport_player_to_checkpoint(&checkpoint, &mut position, &mut transform);

                let consumed = inventory.pop_current();

                debug_assert_eq!(consumed, Some(PlayerAbility::Teleport),);
            }

            Some(PlayerAbility::GravityInvert) => {
                let Some(mut gravity_scale) = gravity_scale else {
                    continue;
                };

                invert_player_gravity(&mut gravity_state, &mut world_gravity, &mut gravity_scale);

                let consumed = inventory.pop_current();

                debug_assert_eq!(consumed, Some(PlayerAbility::GravityInvert),);
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::GridPosition, gameplay::AbilityUseDirection, gameplay::BLOCK_WORLD_SIZE};

    #[test]
    fn queued_teleport_returns_player_to_checkpoint_and_preserves_velocity() {
        let mut app = App::new();

        let checkpoint_position = GridPosition::new(7, -3);

        let mut checkpoint = TeleportCheckpoint::default();
        checkpoint.activate(checkpoint_position);

        app.insert_resource(PlaySession::default())
            .insert_resource(AbilityInventory::default())
            .insert_resource(checkpoint)
            .add_systems(Update, apply_queued_ability_use);

        app.world_mut()
            .resource_mut::<AbilityInventory>()
            .enqueue(PlayerAbility::Teleport);

        let mut intent = AbilityUseIntent::default();
        intent.request(AbilityUseDirection::Right);

        let original_velocity = Vec2::new(4.0, -6.0);

        let player = app
            .world_mut()
            .spawn((
                PlayerBall,
                intent,
                PlayerDashState::default(),
                LinearVelocity(original_velocity),
                Position(Vec2::new(-2.0, 5.0)),
                Transform::from_xyz(-2.0, 5.0, 3.0),
            ))
            .id();

        app.update();

        let expected = Vec2::new(
            checkpoint_position.x as f32 * BLOCK_WORLD_SIZE,
            checkpoint_position.y as f32 * BLOCK_WORLD_SIZE,
        );

        assert_eq!(app.world().get::<Position>(player).unwrap().0, expected,);

        let transform = app.world().get::<Transform>(player).unwrap();

        assert_eq!(
            Vec2::new(transform.translation.x, transform.translation.y,),
            expected,
        );

        // z 좌표는 건드리지 않음.
        assert_eq!(transform.translation.z, 3.0);

        // Unity 원본처럼 기존 속도 유지.
        assert_eq!(
            app.world().get::<LinearVelocity>(player).unwrap().0,
            original_velocity,
        );

        // 사용한 i_tp는 소비됨.
        assert!(app.world().resource::<AbilityInventory>().is_empty());
    }
}
