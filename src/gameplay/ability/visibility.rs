use crate::gameplay::PlayerBall;
use bevy::{color::Alpha, prelude::*};

pub const PLAYER_INVISIBLE_ALPHA: f32 = 0.5;

const PLAYER_VISIBLE_ALPHA: f32 = 1.0;

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PlayerVisibilityState {
    invisible: bool,
}

impl PlayerVisibilityState {
    pub const fn is_invisible(&self) -> bool {
        self.invisible
    }

    pub(crate) fn set_invisible(&mut self, invisible: bool) {
        self.invisible = invisible;
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    const fn alpha(&self) -> f32 {
        if self.invisible {
            PLAYER_INVISIBLE_ALPHA
        } else {
            PLAYER_VISIBLE_ALPHA
        }
    }
}

pub(super) fn sync_player_visibility_visual(
    state: Res<PlayerVisibilityState>,
    mut players: Query<&mut Sprite, With<PlayerBall>>,
) {
    let target_alpha = state.alpha();

    for mut sprite in &mut players {
        if (sprite.color.alpha() - target_alpha).abs() <= f32::EPSILON {
            continue;
        }

        sprite.color.set_alpha(target_alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_state_switches_between_visible_and_invisible() {
        let mut state = PlayerVisibilityState::default();

        assert!(!state.is_invisible());
        assert_eq!(state.alpha(), 1.0);

        state.set_invisible(true);

        assert!(state.is_invisible());
        assert_eq!(state.alpha(), 0.5);

        state.set_invisible(false);

        assert!(!state.is_invisible());
        assert_eq!(state.alpha(), 1.0);
    }
}
