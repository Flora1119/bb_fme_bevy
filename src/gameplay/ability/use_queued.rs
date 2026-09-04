use super::{
    AbilityInventory, AbilityUseIntent, PlayerAbility,
    dash::{PlayerDashState, try_start_dash},
    jump::jump_ability_velocity,
    straight::{ITEM_STRAIGHT_SPEED, straight_ability_direction, straight_ability_velocity},
};
use crate::gameplay::{PlaySession, PlayerBall, StraightMomentum, StraightMovement};
use avian2d::prelude::{GravityScale, LinearVelocity};
use bevy::prelude::*;

pub(super) fn apply_queued_ability_use(
    mut commands: Commands,
    session: Res<PlaySession>,
    mut inventory: ResMut<AbilityInventory>,
    mut players: Query<
        (
            Entity,
            &mut AbilityUseIntent,
            &mut LinearVelocity,
            Option<&mut GravityScale>,
            &mut PlayerDashState,
        ),
        With<PlayerBall>,
    >,
) {
    if !session.is_playing() {
        for (_, mut intent, _, _, _) in &mut players {
            intent.clear();
        }

        return;
    }

    for (player, mut intent, mut velocity, gravity_scale, mut dash_state) in &mut players {
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

            _ => {}
        }
    }
}
