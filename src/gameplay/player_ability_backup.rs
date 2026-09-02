use super::{
    BLOCK_WORLD_SIZE, JumpBlock, MapSpawnSet, PendingPlayInteractions, PlayInteraction,
    PlayInteractionCollectSet, PlayInteractionSet, PlaySession, PlayerBall, PlayerControlInputSet,
    PlayerInputIntent, SpawnValidatedMap, StraightBlock, StraightMomentum, StraightMovement,
};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::VecDeque;

pub const ABILITY_ITEM_SENSOR_RADIUS: f32 = 0.4 * BLOCK_WORLD_SIZE;
pub const ABILITY_DOUBLE_TAP_WINDOW_SECONDS: f32 = 0.3;
pub const ITEM_JUMP_POWER: f32 = 12.0;
pub const ITEM_DASH_SPEED: f32 = 15.0;
pub const ITEM_DASH_DURATION_SECONDS: f32 = 0.15;
pub const ITEM_DASH_JUMP_BOOST: f32 = 3.0;
pub const ITEM_DASH_COOLDOWN_SECONDS: f32 = 0.4;
pub const ITEM_STRAIGHT_SPEED: f32 = 10.0;

const ABILITY_ITEM_SENSOR_COLOR: Color = Color::srgb(0.25, 0.85, 1.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityUseDirection {
    Left,
    Right,
}

impl AbilityUseDirection {
    pub const fn horizontal(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }

    fn from_horizontal(horizontal: f32) -> Option<Self> {
        if horizontal < 0.0 {
            Some(Self::Left)
        } else if horizontal > 0.0 {
            Some(Self::Right)
        } else {
            None
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AbilityUseIntent {
    direction: Option<AbilityUseDirection>,
}

impl AbilityUseIntent {
    pub const fn direction(&self) -> Option<AbilityUseDirection> {
        self.direction
    }

    pub fn request(&mut self, direction: AbilityUseDirection) {
        self.direction = Some(direction);
    }

    pub fn take(&mut self) -> Option<AbilityUseDirection> {
        self.direction.take()
    }

    pub fn clear(&mut self) {
        self.direction = None;
    }
}

#[derive(Resource, Debug, Default)]
pub struct AbilityInventory {
    queue: VecDeque<PlayerAbility>,
}

impl AbilityInventory {
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn current(&self) -> Option<PlayerAbility> {
        self.queue.front().copied()
    }

    pub fn enqueue(&mut self, ability: PlayerAbility) {
        self.queue.push_back(ability);
    }

    pub fn pop_current(&mut self) -> Option<PlayerAbility> {
        self.queue.pop_front()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

#[derive(Resource, Debug, Default)]
struct AbilityDoubleTapState {
    last_direction: Option<AbilityUseDirection>,
    elapsed_seconds: f32,
}

impl AbilityDoubleTapState {
    fn advance(&mut self, delta_seconds: f32) {
        if self.last_direction.is_none() {
            return;
        }

        self.elapsed_seconds += delta_seconds.max(0.0);

        if self.elapsed_seconds > ABILITY_DOUBLE_TAP_WINDOW_SECONDS {
            self.clear();
        }
    }

    fn register_press(&mut self, direction: AbilityUseDirection) -> bool {
        let is_double_tap = self.last_direction == Some(direction)
            && self.elapsed_seconds <= ABILITY_DOUBLE_TAP_WINDOW_SECONDS;

        if is_double_tap {
            // 성공한 더블 탭은 소비합니다.
            //
            // 그래서 Left -> Left -> Left 입력이
            // 두 번째와 세 번째 입력까지 연속 발동시키지 않습니다.
            self.clear();
            true
        } else {
            self.last_direction = Some(direction);
            self.elapsed_seconds = 0.0;
            false
        }
    }

    fn clear(&mut self) {
        self.last_direction = None;
        self.elapsed_seconds = 0.0;
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityItemSensorCollider;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectedAbilityItem;

pub struct PlayerAbilityPlugin;

impl Plugin for PlayerAbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AbilityInventory>()
            .init_resource::<AbilityDoubleTapState>()
            .add_systems(
                Update,
                (
                    reset_ability_state_for_spawn,
                    attach_ability_use_intent,
                    attach_player_dash_state,
                    attach_ability_item_sensors,
                )
                    .after(MapSpawnSet),
            )
            .add_systems(
                PreUpdate,
                (
                    advance_player_dash_state,
                    cancel_active_dash_on_press,
                    capture_ability_use_input,
                    apply_queued_ability_use,
                )
                    .chain()
                    .in_set(PlayerAbilityInputSet)
                    .after(PlayerControlInputSet),
            )
            .add_systems(
                PhysicsSchedule,
                collect_started_ability_item_interactions
                    .in_set(PlayInteractionSet::Collect)
                    .in_set(PlayInteractionCollectSet::Collection),
            );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerAbilityInputSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAbility {
    Jump,
    Dash,
    Straight,
    Teleport,
    GravityInvert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GravityScaleAdjustment {
    Weaker,
    Stronger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityItemEffect {
    Queue(PlayerAbility),
    SetInvisible(bool),
    AdjustGravityScale(GravityScaleAdjustment),
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityItem {
    effect: AbilityItemEffect,
}

impl AbilityItem {
    pub const fn new(effect: AbilityItemEffect) -> Self {
        Self { effect }
    }

    pub const fn effect(self) -> AbilityItemEffect {
        self.effect
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PlayerDashState {
    active_seconds_remaining: f32,
    cooldown_seconds_remaining: f32,
}

impl Default for PlayerDashState {
    fn default() -> Self {
        Self {
            active_seconds_remaining: 0.0,
            cooldown_seconds_remaining: 0.0,
        }
    }
}

impl PlayerDashState {
    pub fn is_active(&self) -> bool {
        self.active_seconds_remaining > 0.0
    }

    pub fn is_on_cooldown(&self) -> bool {
        self.cooldown_seconds_remaining > 0.0
    }

    fn start(&mut self) {
        self.active_seconds_remaining = ITEM_DASH_DURATION_SECONDS;

        self.cooldown_seconds_remaining = ITEM_DASH_COOLDOWN_SECONDS;
    }

    fn cancel(&mut self) {
        self.active_seconds_remaining = 0.0;
    }

    fn advance(&mut self, delta_seconds: f32) {
        let delta_seconds = delta_seconds.max(0.0);

        self.active_seconds_remaining = (self.active_seconds_remaining - delta_seconds).max(0.0);

        self.cooldown_seconds_remaining =
            (self.cooldown_seconds_remaining - delta_seconds).max(0.0);
    }
}

pub fn ability_item_for_id(block_id: &str) -> Option<AbilityItem> {
    let effect = match block_id {
        "i_jump" => AbilityItemEffect::Queue(PlayerAbility::Jump),
        "i_dash" => AbilityItemEffect::Queue(PlayerAbility::Dash),
        "i_st" => AbilityItemEffect::Queue(PlayerAbility::Straight),
        "i_tp" => AbilityItemEffect::Queue(PlayerAbility::Teleport),
        "i_ginvert" => AbilityItemEffect::Queue(PlayerAbility::GravityInvert),

        "i_on" => AbilityItemEffect::SetInvisible(false),
        "i_off" => AbilityItemEffect::SetInvisible(true),

        "i_gup" => AbilityItemEffect::AdjustGravityScale(GravityScaleAdjustment::Weaker),

        "i_gdown" => AbilityItemEffect::AdjustGravityScale(GravityScaleAdjustment::Stronger),

        _ => return None,
    };

    Some(AbilityItem::new(effect))
}

fn jump_ability_velocity(current: Vec2) -> Vec2 {
    Vec2::new(current.x, ITEM_JUMP_POWER)
}

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

fn attach_ability_item_sensors(
    mut commands: Commands,
    items: Query<
        (Entity, &AbilityItem),
        (
            Without<CollectedAbilityItem>,
            Without<AbilityItemSensorCollider>,
        ),
    >,
) {
    for (entity, item) in &items {
        // 이번 단계에서는 Queue형 아이템만 실제 획득 가능합니다.
        //
        // i_on / i_off / i_gup / i_gdown은
        // 5-B-1.8에서 즉시 적용형으로 구현합니다.
        if !matches!(item.effect(), AbilityItemEffect::Queue(_)) {
            continue;
        }

        commands.entity(entity).insert((
            AbilityItemSensorCollider,
            Sensor,
            CollisionEventsEnabled,
            Collider::circle(ABILITY_ITEM_SENSOR_RADIUS),
            DebugRender::default().with_collider_color(ABILITY_ITEM_SENSOR_COLOR),
        ));
    }
}

fn collect_started_ability_item_interactions(
    mut collision_starts: MessageReader<CollisionStart>,
    players: Query<(), With<PlayerBall>>,
    items: Query<
        (),
        (
            With<AbilityItem>,
            With<AbilityItemSensorCollider>,
            Without<CollectedAbilityItem>,
        ),
    >,
    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let item = if players.contains(event.collider1) && items.contains(event.collider2) {
            event.collider2
        } else if players.contains(event.collider2) && items.contains(event.collider1) {
            event.collider1
        } else {
            continue;
        };

        pending.push(PlayInteraction::collection(item));
    }
}

fn capture_ability_use_input(
    session: Res<PlaySession>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    inventory: Res<AbilityInventory>,
    mut double_tap: ResMut<AbilityDoubleTapState>,
    mut players: Query<(&PlayerInputIntent, &mut AbilityUseIntent), With<PlayerBall>>,
) {
    // Intent는 한 프레임짜리 요청입니다.
    for (_, mut ability_intent) in &mut players {
        ability_intent.clear();
    }

    // 플레이 중이 아니거나 보유 능력이 없다면
    // 이전 탭 기록도 남기지 않습니다.
    //
    // 예:
    // 아이템 없는 상태에서 Left
    // -> 바로 i_jump 획득
    // -> Left
    //
    // 이것을 더블 탭으로 오인하면 안 됩니다.
    if !session.is_playing() || inventory.is_empty() {
        double_tap.clear();
        return;
    }

    double_tap.advance(time.delta_secs());

    let left_just_pressed =
        keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA);

    let right_just_pressed =
        keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);

    let double_tap_direction = match (left_just_pressed, right_just_pressed) {
        (true, false) => {
            let direction = AbilityUseDirection::Left;

            double_tap.register_press(direction).then_some(direction)
        }

        (false, true) => {
            let direction = AbilityUseDirection::Right;

            double_tap.register_press(direction).then_some(direction)
        }

        (true, true) => {
            // 좌우가 동시에 눌렸다면 방향이 모호하므로
            // 더블 탭 기록을 취소합니다.
            double_tap.clear();
            None
        }

        (false, false) => None,
    };

    let space_just_pressed = keyboard.just_pressed(KeyCode::Space);

    for (movement_intent, mut ability_intent) in &mut players {
        // 원작의 보조 입력:
        //
        // 방향키를 누른 상태에서 Space를 누르면
        // 더블 탭 없이도 현재 능력을 사용합니다.
        let space_direction = if space_just_pressed {
            AbilityUseDirection::from_horizontal(movement_intent.horizontal())
        } else {
            None
        };

        // 같은 프레임에 둘 다 발생하면
        // 더블 탭 입력을 우선합니다.
        if let Some(direction) = double_tap_direction.or(space_direction) {
            ability_intent.request(direction);
        }
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

fn dash_ability_velocity(current: Vec2, direction: AbilityUseDirection) -> Vec2 {
    Vec2::new(
        ITEM_DASH_SPEED * direction.horizontal(),
        current.y.max(ITEM_DASH_JUMP_BOOST),
    )
}

fn cancelled_dash_velocity(current: Vec2) -> Vec2 {
    Vec2::new(current.x * 0.5, current.y)
}

fn try_start_dash(
    dash_state: &mut PlayerDashState,
    velocity: &mut LinearVelocity,
    direction: AbilityUseDirection,
) -> bool {
    if dash_state.is_on_cooldown() {
        return false;
    }

    velocity.0 = dash_ability_velocity(velocity.0, direction);

    dash_state.start();

    true
}

fn advance_player_dash_state(
    time: Res<Time>,
    mut players: Query<&mut PlayerDashState, With<PlayerBall>>,
) {
    let delta_seconds = time.delta_secs();

    for mut dash_state in &mut players {
        dash_state.advance(delta_seconds);
    }
}

fn cancel_active_dash_on_press(
    session: Res<PlaySession>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&mut PlayerDashState, &mut LinearVelocity), With<PlayerBall>>,
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

    for (mut dash_state, mut velocity) in &mut players {
        if !dash_state.is_active() {
            continue;
        }

        dash_state.cancel();

        velocity.0 = cancelled_dash_velocity(velocity.0);
    }
}

fn straight_ability_direction(direction: AbilityUseDirection) -> Vec2 {
    Vec2::new(direction.horizontal(), 0.0)
}

fn straight_ability_velocity(direction: AbilityUseDirection) -> Vec2 {
    straight_ability_direction(direction) * ITEM_STRAIGHT_SPEED
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
