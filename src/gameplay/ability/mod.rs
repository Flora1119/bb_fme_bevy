mod dash;
mod gravity;
mod input;
mod inventory;
mod item_collection;
mod jump;
mod straight;
mod teleport;
mod types;
mod use_queued;
mod visibility;

use super::{MapSpawnSet, PlayInteractionCollectSet, PlayInteractionSet, PlayerControlInputSet};
use avian2d::prelude::PhysicsSchedule;
use bevy::prelude::*;

pub use gravity::*;
pub use inventory::*;
pub use item_collection::*;
pub use teleport::*;
pub use types::*;
pub use visibility::*;

pub struct PlayerAbilityPlugin;

impl Plugin for PlayerAbilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AbilityInventory>()
            .init_resource::<TeleportCheckpoint>()
            .init_resource::<PlayerGravityState>()
            .init_resource::<PlayerVisibilityState>()
            .init_resource::<input::AbilityDoubleTapState>()
            .add_systems(
                Update,
                (
                    input::reset_ability_state_for_spawn,
                    input::attach_ability_use_intent,
                    dash::attach_player_dash_state,
                    item_collection::attach_ability_item_sensors,
                    visibility::sync_player_visibility_visual,
                )
                    .after(MapSpawnSet),
            )
            .add_systems(
                PreUpdate,
                (
                    dash::advance_player_dash_state,
                    dash::cancel_active_dash_on_press,
                    input::capture_ability_use_input,
                    use_queued::apply_queued_ability_use,
                )
                    .chain()
                    .in_set(input::PlayerAbilityInputSet)
                    .after(PlayerControlInputSet),
            )
            .add_systems(
                PhysicsSchedule,
                item_collection::collect_started_ability_item_interactions
                    .in_set(PlayInteractionSet::Collect)
                    .in_set(PlayInteractionCollectSet::Collection),
            );
    }
}
