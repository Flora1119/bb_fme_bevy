use super::{AbilityItem, AbilityItemEffect, GravityScaleAdjustment, PlayerAbility};

use crate::gameplay::{BLOCK_WORLD_SIZE, PendingPlayInteractions, PlayInteraction, PlayerBall};

use avian2d::prelude::*;
use bevy::prelude::*;

pub const ABILITY_ITEM_SENSOR_RADIUS: f32 = 0.4 * BLOCK_WORLD_SIZE;

const ABILITY_ITEM_SENSOR_COLOR: Color = Color::srgb(0.25, 0.85, 1.0);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityItemSensorCollider;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectedAbilityItem;

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

pub(super) fn attach_ability_item_sensors(
    mut commands: Commands,
    items: Query<
        (Entity, &AbilityItem),
        (
            Without<CollectedAbilityItem>,
            Without<AbilityItemSensorCollider>,
        ),
    >,
) {
    for (entity, _) in &items {
        commands.entity(entity).insert((
            AbilityItemSensorCollider,
            Sensor,
            CollisionEventsEnabled,
            Collider::circle(ABILITY_ITEM_SENSOR_RADIUS),
            DebugRender::default().with_collider_color(ABILITY_ITEM_SENSOR_COLOR),
        ));
    }
}

pub(super) fn collect_started_ability_item_interactions(
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
