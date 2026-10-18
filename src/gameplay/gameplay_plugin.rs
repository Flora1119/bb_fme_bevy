use super::{
    ChangeBlockPlugin, ClockBlockPlugin, FallingRockPlugin, GameplayPhysicsPlugin,
    MapBoundaryPlugin, MapSpawnPlugin, PlayRestartPlugin, PlaySessionPlugin, PlayerAbilityPlugin,
    PlayerControlPlugin, SpikeDeathPlugin, StarCollectionPlugin, StarSwitchBlockPlugin,
    StraightTransportPlugin, SwitchBlockPlugin, TeleportBlockPlugin,
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
            StraightTransportPlugin,
            PlayerAbilityPlugin,
            PlayerControlPlugin,
        ));
        app.add_plugins(FallingRockPlugin);
    }
}
