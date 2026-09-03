use super::{
    BLOCK_WORLD_SIZE, JumpBlock, MapSpawnSet, PendingPlayInteractions, PlayInteraction,
    PlayInteractionCollectSet, PlayInteractionSet, PlaySession, PlayerBall, PlayerControlInputSet,
    PlayerInputIntent, SpawnValidatedMap, StraightBlock, StraightMomentum, StraightMovement,
};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerAbilityInputSet;

fn attach_ability_use_intent(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerBall>, Without<AbilityUseIntent>)>,
) {
    for player in &players {
        commands.entity(player).insert(AbilityUseIntent::default());
    }
}

fn reset_ability_state_for_spawn(
    mut spawn_requests: MessageReader<SpawnValidatedMap>,
    mut inventory: ResMut<AbilityInventory>,
    mut double_tap: ResMut<AbilityDoubleTapState>,
) {
    if spawn_requests.read().next().is_some() {
        inventory.clear();
        double_tap.clear();
    }
}

fn apply_queued_ability_use(
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

                debug_assert_eq!(consumed, Some(PlayerAbility::Jump),);
            }

            Some(PlayerAbility::Dash) => {
                try_start_dash(&mut dash_state, &mut velocity, direction);

                // Unity 원본 호환:
                // 쿨타임으로 실제 Dash가 발동되지 않아도
                // 현재 i_dash는 소비됩니다.
                let consumed = inventory.pop_current();

                debug_assert_eq!(consumed, Some(PlayerAbility::Dash),);
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

                debug_assert_eq!(consumed, Some(PlayerAbility::Straight),);
            }

            _ => {}
        }
    }
}

fn attach_player_dash_state(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerBall>, Without<PlayerDashState>)>,
) {
    for player in &players {
        commands.entity(player).insert(PlayerDashState::default());
    }
}

fn cancelled_dash_velocity(current: Vec2) -> Vec2 {
    Vec2::new(current.x * 0.5, current.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_ability_items_map_to_expected_effects() {
        let cases = [
            ("i_jump", AbilityItemEffect::Queue(PlayerAbility::Jump)),
            ("i_dash", AbilityItemEffect::Queue(PlayerAbility::Dash)),
            ("i_st", AbilityItemEffect::Queue(PlayerAbility::Straight)),
            ("i_tp", AbilityItemEffect::Queue(PlayerAbility::Teleport)),
            (
                "i_ginvert",
                AbilityItemEffect::Queue(PlayerAbility::GravityInvert),
            ),
            ("i_on", AbilityItemEffect::SetInvisible(false)),
            ("i_off", AbilityItemEffect::SetInvisible(true)),
            (
                "i_gup",
                AbilityItemEffect::AdjustGravityScale(GravityScaleAdjustment::Weaker),
            ),
            (
                "i_gdown",
                AbilityItemEffect::AdjustGravityScale(GravityScaleAdjustment::Stronger),
            ),
        ];

        for (block_id, expected_effect) in cases {
            let item = ability_item_for_id(block_id)
                .unwrap_or_else(|| panic!("{block_id} should be mapped"));

            assert_eq!(
                item.effect(),
                expected_effect,
                "wrong effect for {block_id}",
            );
        }
    }

    #[test]
    fn complex_items_are_left_unmapped_until_their_systems_exist() {
        for block_id in ["i_wall", "i_circle", "i_clone", "i_swing"] {
            assert_eq!(
                ability_item_for_id(block_id),
                None,
                "{block_id} must stay inactive for now",
            );
        }
    }

    #[test]
    fn non_ability_items_are_not_mapped() {
        for block_id in [
            "ball",
            "star",
            "star_empty",
            "star_jump",
            "b_normal",
            "unknown",
        ] {
            assert_eq!(
                ability_item_for_id(block_id),
                None,
                "{block_id} must not become an AbilityItem",
            );
        }
    }

    #[test]
    fn ability_inventory_is_fifo() {
        let mut inventory = AbilityInventory::default();

        assert!(inventory.is_empty());
        assert_eq!(inventory.current(), None);

        inventory.enqueue(PlayerAbility::Jump);
        inventory.enqueue(PlayerAbility::Straight);
        inventory.enqueue(PlayerAbility::Teleport);

        assert_eq!(inventory.len(), 3);
        assert_eq!(inventory.current(), Some(PlayerAbility::Jump));

        assert_eq!(inventory.pop_current(), Some(PlayerAbility::Jump),);

        assert_eq!(inventory.pop_current(), Some(PlayerAbility::Straight),);

        assert_eq!(inventory.pop_current(), Some(PlayerAbility::Teleport),);

        assert!(inventory.is_empty());
    }

    #[test]
    fn same_direction_within_window_is_a_double_tap() {
        let mut state = AbilityDoubleTapState::default();

        assert!(!state.register_press(AbilityUseDirection::Left));

        state.advance(0.2);

        assert!(state.register_press(AbilityUseDirection::Left));
    }

    #[test]
    fn same_direction_after_window_is_not_a_double_tap() {
        let mut state = AbilityDoubleTapState::default();

        assert!(!state.register_press(AbilityUseDirection::Right));

        state.advance(ABILITY_DOUBLE_TAP_WINDOW_SECONDS + 0.01);

        assert!(!state.register_press(AbilityUseDirection::Right));
    }

    #[test]
    fn opposite_direction_restarts_the_double_tap_sequence() {
        let mut state = AbilityDoubleTapState::default();

        assert!(!state.register_press(AbilityUseDirection::Left));

        state.advance(0.1);

        assert!(!state.register_press(AbilityUseDirection::Right));

        state.advance(0.1);

        assert!(!state.register_press(AbilityUseDirection::Left));
    }

    #[test]
    fn successful_double_tap_consumes_the_sequence() {
        let mut state = AbilityDoubleTapState::default();

        assert!(!state.register_press(AbilityUseDirection::Left));

        state.advance(0.1);

        assert!(state.register_press(AbilityUseDirection::Left));

        state.advance(0.1);

        // 직전 더블 탭이 이미 소비되었으므로
        // 이 Left는 새로운 첫 번째 탭입니다.
        assert!(!state.register_press(AbilityUseDirection::Left));
    }

    #[test]
    fn horizontal_input_maps_to_ability_direction() {
        assert_eq!(
            AbilityUseDirection::from_horizontal(-1.0),
            Some(AbilityUseDirection::Left),
        );

        assert_eq!(
            AbilityUseDirection::from_horizontal(1.0),
            Some(AbilityUseDirection::Right),
        );

        assert_eq!(AbilityUseDirection::from_horizontal(0.0), None,);
    }

    #[test]
    fn jump_ability_sets_vertical_speed_and_preserves_horizontal_speed() {
        assert_eq!(
            jump_ability_velocity(Vec2::new(4.0, -7.0)),
            Vec2::new(4.0, ITEM_JUMP_POWER),
        );

        assert_eq!(
            jump_ability_velocity(Vec2::new(-3.0, 8.0)),
            Vec2::new(-3.0, ITEM_JUMP_POWER),
        );
    }

    #[test]
    fn jump_item_uses_the_original_item_power_not_jump_block_power() {
        assert_eq!(ITEM_JUMP_POWER, 12.0);

        // 기능 블록은 별도 밸런스 값입니다.
        assert_ne!(ITEM_JUMP_POWER, JumpBlock::STANDARD_LAUNCH_SPEED,);
    }

    #[test]
    fn queued_jump_is_used_and_removed_from_inventory() {
        let mut app = App::new();

        app.insert_resource(PlaySession::default())
            .insert_resource(AbilityInventory::default())
            .add_systems(Update, apply_queued_ability_use);

        app.world_mut()
            .resource_mut::<AbilityInventory>()
            .enqueue(PlayerAbility::Jump);

        let mut intent = AbilityUseIntent::default();
        intent.request(AbilityUseDirection::Left);

        let player = app
            .world_mut()
            .spawn((
                PlayerBall,
                intent,
                PlayerDashState::default(),
                LinearVelocity(Vec2::new(4.0, -7.0)),
            ))
            .id();

        app.update();

        let velocity = app
            .world()
            .get::<LinearVelocity>(player)
            .expect("player must keep LinearVelocity")
            .0;

        assert_eq!(velocity, Vec2::new(4.0, ITEM_JUMP_POWER),);

        assert!(app.world().resource::<AbilityInventory>().is_empty());

        assert_eq!(
            app.world()
                .get::<AbilityUseIntent>(player)
                .expect("player must keep AbilityUseIntent")
                .direction(),
            None,
        );
    }

    #[test]
    fn unsupported_queued_ability_is_not_consumed_yet() {
        let mut app = App::new();

        app.insert_resource(PlaySession::default())
            .insert_resource(AbilityInventory::default())
            .add_systems(Update, apply_queued_ability_use);

        app.world_mut()
            .resource_mut::<AbilityInventory>()
            .enqueue(PlayerAbility::Teleport);

        let mut intent = AbilityUseIntent::default();
        intent.request(AbilityUseDirection::Right);

        let player = app
            .world_mut()
            .spawn((
                PlayerBall,
                intent,
                PlayerDashState::default(),
                LinearVelocity(Vec2::new(2.0, 3.0)),
            ))
            .id();

        app.update();

        assert_eq!(
            app.world().resource::<AbilityInventory>().current(),
            Some(PlayerAbility::Teleport),
        );

        assert_eq!(
            app.world().get::<LinearVelocity>(player).unwrap().0,
            Vec2::new(2.0, 3.0),
        );
    }

    #[test]
    fn dash_sets_horizontal_speed_and_minimum_vertical_boost() {
        assert_eq!(
            dash_ability_velocity(Vec2::new(2.0, -7.0), AbilityUseDirection::Right,),
            Vec2::new(ITEM_DASH_SPEED, ITEM_DASH_JUMP_BOOST,),
        );

        assert_eq!(
            dash_ability_velocity(Vec2::new(-4.0, 8.0), AbilityUseDirection::Left,),
            Vec2::new(-ITEM_DASH_SPEED, 8.0),
        );
    }

    #[test]
    fn cancelling_dash_halves_only_horizontal_velocity() {
        assert_eq!(
            cancelled_dash_velocity(Vec2::new(15.0, 6.0)),
            Vec2::new(7.5, 6.0),
        );

        assert_eq!(
            cancelled_dash_velocity(Vec2::new(-15.0, -2.0)),
            Vec2::new(-7.5, -2.0),
        );
    }

    #[test]
    fn dash_state_tracks_active_duration_and_cooldown() {
        let mut state = PlayerDashState::default();

        state.start();

        assert!(state.is_active());
        assert!(state.is_on_cooldown());

        state.advance(ITEM_DASH_DURATION_SECONDS);

        assert!(!state.is_active());
        assert!(state.is_on_cooldown());

        state.advance(ITEM_DASH_COOLDOWN_SECONDS - ITEM_DASH_DURATION_SECONDS);

        assert!(!state.is_active());
        assert!(!state.is_on_cooldown());
    }

    #[test]
    fn queued_dash_is_used_and_removed_from_inventory() {
        let mut app = App::new();

        app.insert_resource(PlaySession::default())
            .insert_resource(AbilityInventory::default())
            .add_systems(Update, apply_queued_ability_use);

        app.world_mut()
            .resource_mut::<AbilityInventory>()
            .enqueue(PlayerAbility::Dash);

        let mut intent = AbilityUseIntent::default();
        intent.request(AbilityUseDirection::Right);

        let player = app
            .world_mut()
            .spawn((
                PlayerBall,
                intent,
                PlayerDashState::default(),
                LinearVelocity(Vec2::new(2.0, -7.0)),
            ))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<LinearVelocity>(player).unwrap().0,
            Vec2::new(ITEM_DASH_SPEED, ITEM_DASH_JUMP_BOOST,),
        );

        assert!(
            app.world()
                .get::<PlayerDashState>(player)
                .unwrap()
                .is_active()
        );

        assert!(app.world().resource::<AbilityInventory>().is_empty());
    }

    #[test]
    fn dash_on_cooldown_is_consumed_without_changing_velocity() {
        let mut app = App::new();

        app.insert_resource(PlaySession::default())
            .insert_resource(AbilityInventory::default())
            .add_systems(Update, apply_queued_ability_use);

        app.world_mut()
            .resource_mut::<AbilityInventory>()
            .enqueue(PlayerAbility::Dash);

        let mut intent = AbilityUseIntent::default();
        intent.request(AbilityUseDirection::Left);

        let mut dash_state = PlayerDashState::default();

        // 이미 한 번 Dash한 뒤
        // active 상태만 종료됐고 cooldown은 남은 상태.
        dash_state.start();
        dash_state.cancel();

        let original_velocity = Vec2::new(4.0, 6.0);

        let player = app
            .world_mut()
            .spawn((
                PlayerBall,
                intent,
                dash_state,
                LinearVelocity(original_velocity),
            ))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<LinearVelocity>(player).unwrap().0,
            original_velocity,
        );

        // 효과는 발생하지 않았지만
        // Unity 원본처럼 i_dash는 소비됩니다.
        assert!(app.world().resource::<AbilityInventory>().is_empty());
    }

    #[test]
    fn straight_ability_uses_original_item_speed() {
        assert_eq!(ITEM_STRAIGHT_SPEED, 10.0);

        assert_ne!(ITEM_STRAIGHT_SPEED, StraightBlock::STANDARD_SPEED,);

        assert_ne!(ITEM_STRAIGHT_SPEED, StraightBlock::HIGH_SPEED,);
    }

    #[test]
    fn straight_ability_replaces_velocity_with_horizontal_motion() {
        assert_eq!(
            straight_ability_velocity(AbilityUseDirection::Right,),
            Vec2::new(ITEM_STRAIGHT_SPEED, 0.0),
        );

        assert_eq!(
            straight_ability_velocity(AbilityUseDirection::Left,),
            Vec2::new(-ITEM_STRAIGHT_SPEED, 0.0),
        );
    }

    #[test]
    fn queued_straight_starts_cancelable_straight_movement() {
        let mut app = App::new();

        app.insert_resource(PlaySession::default())
            .insert_resource(AbilityInventory::default())
            .add_systems(Update, apply_queued_ability_use);

        {
            let mut inventory = app.world_mut().resource_mut::<AbilityInventory>();

            inventory.enqueue(PlayerAbility::Straight);
            inventory.enqueue(PlayerAbility::Jump);
        }

        let mut intent = AbilityUseIntent::default();
        intent.request(AbilityUseDirection::Right);

        let player = app
            .world_mut()
            .spawn((
                PlayerBall,
                intent,
                PlayerDashState::default(),
                LinearVelocity(Vec2::new(-4.0, 8.0)),
                GravityScale(3.0),
            ))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<LinearVelocity>(player).unwrap().0,
            Vec2::new(ITEM_STRAIGHT_SPEED, 0.0),
        );

        assert_eq!(app.world().get::<GravityScale>(player).unwrap().0, 0.0,);

        let straight = app
            .world()
            .get::<StraightMovement>(player)
            .expect("i_st must start StraightMovement");

        assert_eq!(straight.direction(), Vec2::X,);

        assert_eq!(straight.speed(), ITEM_STRAIGHT_SPEED,);

        assert!(
            straight.can_cancel_on_press(),
            "item straight must be cancelable by press",
        );

        // FIFO 다음 능력은 그대로 남아야 함.
        assert_eq!(
            app.world().resource::<AbilityInventory>().current(),
            Some(PlayerAbility::Jump),
        );
    }

    #[test]
    fn straight_ability_replaces_existing_straight_momentum() {
        let mut app = App::new();

        app.insert_resource(PlaySession::default())
            .insert_resource(AbilityInventory::default())
            .add_systems(Update, apply_queued_ability_use);

        app.world_mut()
            .resource_mut::<AbilityInventory>()
            .enqueue(PlayerAbility::Straight);

        let mut intent = AbilityUseIntent::default();
        intent.request(AbilityUseDirection::Left);

        let player = app
            .world_mut()
            .spawn((
                PlayerBall,
                intent,
                PlayerDashState::default(),
                LinearVelocity(Vec2::new(5.0, 4.0)),
                GravityScale(3.0),
                StraightMomentum::new(Vec2::X, 12.0, 0.3),
            ))
            .id();

        app.update();

        assert!(app.world().get::<StraightMomentum>(player).is_none());

        assert!(app.world().get::<StraightMovement>(player).is_some());

        assert_eq!(
            app.world().get::<LinearVelocity>(player).unwrap().0,
            Vec2::new(-ITEM_STRAIGHT_SPEED, 0.0),
        );
    }
}
