use super::AbilityUseDirection;
use crate::gameplay::{PlaySession, PlayerBall};
use avian2d::prelude::LinearVelocity;
use bevy::prelude::*;

pub const ITEM_DASH_SPEED: f32 = 15.0;
pub const ITEM_DASH_DURATION_SECONDS: f32 = 0.15;
pub const ITEM_DASH_JUMP_BOOST: f32 = 3.0;
pub const ITEM_DASH_COOLDOWN_SECONDS: f32 = 0.4;

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

fn dash_ability_velocity(current: Vec2, direction: AbilityUseDirection) -> Vec2 {
    Vec2::new(
        ITEM_DASH_SPEED * direction.horizontal(),
        current.y.max(ITEM_DASH_JUMP_BOOST),
    )
}

pub(super) fn try_start_dash(
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

fn cancelled_dash_velocity(current: Vec2) -> Vec2 {
    Vec2::new(current.x * 0.5, current.y)
}

pub(super) fn advance_player_dash_state(
    time: Res<Time>,
    mut players: Query<&mut PlayerDashState, With<PlayerBall>>,
) {
    let delta_seconds = time.delta_secs();

    for mut dash_state in &mut players {
        dash_state.advance(delta_seconds);
    }
}

pub(super) fn cancel_active_dash_on_press(
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

pub(super) fn attach_player_dash_state(
    mut commands: Commands,
    players: Query<Entity, (With<PlayerBall>, Without<PlayerDashState>)>,
) {
    for player in &players {
        commands.entity(player).insert(PlayerDashState::default());
    }
}
