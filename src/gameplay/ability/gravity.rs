use crate::gameplay::{PLAYER_GRAVITY_SCALE, WORLD_GRAVITY};
use avian2d::prelude::{Gravity, GravityScale};
use bevy::prelude::*;

use super::GravityScaleAdjustment;

pub const ITEM_GRAVITY_SCALE_STEP: f32 = 1.5;
pub const ITEM_GRAVITY_SCALE_MIN: f32 = 0.5;
pub const ITEM_GRAVITY_SCALE_MAX_STRONGER: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerGravityDirection {
    Down,
    Up,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlayerGravityState {
    direction: PlayerGravityDirection,
    scale: f32,
}

impl Default for PlayerGravityState {
    fn default() -> Self {
        Self {
            direction: PlayerGravityDirection::Down,
            scale: PLAYER_GRAVITY_SCALE,
        }
    }
}

impl PlayerGravityState {
    pub const fn direction(&self) -> PlayerGravityDirection {
        self.direction
    }

    pub const fn scale(&self) -> f32 {
        self.scale
    }

    pub fn world_gravity(&self) -> Vec2 {
        match self.direction {
            PlayerGravityDirection::Down => WORLD_GRAVITY,
            PlayerGravityDirection::Up => -WORLD_GRAVITY,
        }
    }

    pub(super) fn invert(&mut self) {
        self.direction = match self.direction {
            PlayerGravityDirection::Down => PlayerGravityDirection::Up,
            PlayerGravityDirection::Up => PlayerGravityDirection::Down,
        };
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn adjust_scale(&mut self, adjustment: GravityScaleAdjustment) {
        self.scale = match adjustment {
            GravityScaleAdjustment::Weaker => {
                (self.scale - ITEM_GRAVITY_SCALE_STEP).max(ITEM_GRAVITY_SCALE_MIN)
            }

            GravityScaleAdjustment::Stronger => {
                (self.scale + ITEM_GRAVITY_SCALE_STEP).min(ITEM_GRAVITY_SCALE_MAX_STRONGER)
            }
        };
    }
}

pub(super) fn invert_player_gravity(
    state: &mut PlayerGravityState,
    world_gravity: &mut Gravity,
    player_gravity_scale: &mut GravityScale,
) {
    state.invert();

    world_gravity.0 = state.world_gravity();

    // Unity 원본의 ApplyGravityScale(true) 대응.
    //
    // Straight 등에 의해 일시적으로 0이 되어 있더라도
    // 현재 게임 중력 배율을 다시 적용합니다.
    player_gravity_scale.0 = state.scale();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_inversion_flips_world_direction_and_restores_scale() {
        let mut state = PlayerGravityState::default();

        let mut world_gravity = Gravity(WORLD_GRAVITY);

        // Straight 사용 중처럼 중력이 일시적으로 꺼진 상황을 가정.
        let mut gravity_scale = GravityScale(0.0);

        invert_player_gravity(&mut state, &mut world_gravity, &mut gravity_scale);

        assert_eq!(state.direction(), PlayerGravityDirection::Up,);

        assert_eq!(world_gravity.0, -WORLD_GRAVITY,);

        assert_eq!(gravity_scale.0, PLAYER_GRAVITY_SCALE,);

        invert_player_gravity(&mut state, &mut world_gravity, &mut gravity_scale);

        assert_eq!(state.direction(), PlayerGravityDirection::Down,);

        assert_eq!(world_gravity.0, WORLD_GRAVITY,);
    }

    #[test]
    fn gravity_scale_adjustment_matches_original_item_limits() {
        let mut state = PlayerGravityState::default();

        assert_eq!(state.scale(), 3.0);

        state.adjust_scale(GravityScaleAdjustment::Weaker);

        assert_eq!(state.scale(), 1.5);

        state.adjust_scale(GravityScaleAdjustment::Weaker);

        assert_eq!(state.scale(), 0.5);

        state.adjust_scale(GravityScaleAdjustment::Weaker);

        assert_eq!(state.scale(), 0.5);

        state.reset();

        assert_eq!(state.scale(), 3.0);

        state.adjust_scale(GravityScaleAdjustment::Stronger);

        assert_eq!(state.scale(), 4.5);

        state.adjust_scale(GravityScaleAdjustment::Stronger);

        assert_eq!(state.scale(), 6.0);

        state.adjust_scale(GravityScaleAdjustment::Stronger);

        assert_eq!(state.scale(), 6.0);
    }
}
