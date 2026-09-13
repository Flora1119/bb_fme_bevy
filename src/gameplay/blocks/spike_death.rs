use crate::gameplay::{
    DeadlySpike, ElectricControlledBlock, PendingPlayInteractions, PlayInteraction,
    PlayInteractionCollectSet, PlayInteractionSet, PlayerBall, SpikeSensorCollider,
};
use avian2d::prelude::*;
use bevy::prelude::*;

pub struct SpikeDeathPlugin;

impl Plugin for SpikeDeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PhysicsSchedule,
            (
                collect_started_spike_interactions,
                collect_started_electric_hazard_interactions,
            )
                .in_set(PlayInteractionSet::Collect)
                .in_set(PlayInteractionCollectSet::Death),
        );
    }
}

fn collect_started_spike_interactions(
    mut collision_starts: MessageReader<CollisionStart>,
    players: Query<(), With<PlayerBall>>,
    spike_sensors: Query<&ChildOf, With<SpikeSensorCollider>>,
    spikes: Query<(), With<DeadlySpike>>,
    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let spike_sensor =
            if players.contains(event.collider1) && spike_sensors.contains(event.collider2) {
                event.collider2
            } else if players.contains(event.collider2) && spike_sensors.contains(event.collider1) {
                event.collider1
            } else {
                continue;
            };

        let Ok(child_of) = spike_sensors.get(spike_sensor) else {
            continue;
        };

        let spike = child_of.parent();

        if !spikes.contains(spike) {
            continue;
        }

        pending.push(PlayInteraction::death(spike));
    }
}

fn collect_started_electric_hazard_interactions(
    mut collision_starts: MessageReader<CollisionStart>,
    players: Query<(), With<PlayerBall>>,
    electric_blocks: Query<&ElectricControlledBlock>,
    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let hazard = if players.contains(event.collider1)
            && electric_blocks.contains(event.collider2)
        {
            event.collider2
        } else if players.contains(event.collider2) && electric_blocks.contains(event.collider1) {
            event.collider1
        } else {
            continue;
        };

        let Ok(electric_block) = electric_blocks.get(hazard) else {
            continue;
        };

        if *electric_block != ElectricControlledBlock::Hazard {
            continue;
        }

        pending.push(PlayInteraction::death(hazard));
    }
}
