mod dash;
mod input;
mod inventory;
mod item_collection;
mod jump;
mod straight;
mod types;
mod use_queued;

pub use inventory::*;
pub use item_collection::*;
pub use types::*;

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
