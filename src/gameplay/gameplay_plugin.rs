use super::{
    ClockBlockPlugin, GameplayPhysicsPlugin, MapBoundaryPlugin, MapSpawnPlugin, PlayRestartPlugin,
    PlaySessionPlugin, PlayerAbilityPlugin, PlayerControlPlugin, SpikeDeathPlugin,
    StarCollectionPlugin, TeleportBlockPlugin,
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
            StarCollectionPlugin,
            SpikeDeathPlugin,
            ClockBlockPlugin,
            TeleportBlockPlugin,
            PlayerAbilityPlugin,
            PlayerControlPlugin,
        ));
    }
}
