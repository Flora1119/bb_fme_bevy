use super::{
    ChangeBlockPlugin, ClockBlockPlugin, GameplayPhysicsPlugin, MapBoundaryPlugin, MapSpawnPlugin,
    PlayRestartPlugin, PlaySessionPlugin, PlayerAbilityPlugin, PlayerControlPlugin,
    SpikeDeathPlugin, StarCollectionPlugin, StarSwitchBlockPlugin, SwitchBlockPlugin,
    TeleportBlockPlugin,
};
use bevy::prelude::*;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MapSpawnPlugin,
            GameplayPhysicsPlugin,
            PlaySessionPlugin,
            PlayRestartPlugin,
            MapBoundaryPlugin,
            StarSwitchBlockPlugin,
            StarCollectionPlugin,
            SpikeDeathPlugin,
            ClockBlockPlugin,
            ChangeBlockPlugin,
            TeleportBlockPlugin,
            SwitchBlockPlugin,
            PlayerAbilityPlugin,
            PlayerControlPlugin,
        ));
    }
}
