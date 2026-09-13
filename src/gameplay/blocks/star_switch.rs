use crate::gameplay::{
    BLOCK_WORLD_SIZE, BlockVisualSet, CollectedStar, MapSpawnSet, PlayWorld, PlayerBall,
    StarSensorCollider, TransparentStar,
};
use avian2d::prelude::*;
use bevy::prelude::*;
use std::collections::HashSet;

pub const STAR_SWITCH_SENSOR_SIZE: f32 = BLOCK_WORLD_SIZE;

const STAR_ACTIVE_VISUAL_PATH: &str = "sprites/item/star.png";

const STAR_INACTIVE_VISUAL_PATH: &str = "sprites/item/star_empty.png";

const STAR_SWITCH_ICON_SCALE: f32 = 0.5;
const STAR_SWITCH_ICON_Z: f32 = 0.01;
const STAR_SWITCH_SENSOR_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct StarSwitchVisualAttached;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct StarSwitchVisual;

#[derive(Component, Debug, Clone)]
struct TransparentStarVisualHandles {
    active: Handle<Image>,
    inactive: Handle<Image>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarSwitchTrigger;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarSwitchSensor;

#[derive(Resource, Debug, Default)]
struct ActiveStarSwitchContacts(HashSet<(Entity, Entity)>);

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StarSwitchState {
    active: bool,
}

impl StarSwitchState {
    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn reset(&mut self) {
        self.active = false;
    }
}

pub struct StarSwitchBlockPlugin;

impl Plugin for StarSwitchBlockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StarSwitchState>()
            .init_resource::<ActiveStarSwitchContacts>()
            .add_systems(Update, attach_star_switch_sensors.after(MapSpawnSet))
            .add_systems(
                Update,
                reset_star_switch_state_on_play_world_spawn.after(MapSpawnSet),
            )
            .add_systems(
                PhysicsSchedule,
                update_star_switch_contacts
                    .after(PhysicsStepSystems::NarrowPhase)
                    .before(PhysicsStepSystems::Solver),
            )
            .add_systems(
                Update,
                prepare_transparent_star_visual_handles.after(MapSpawnSet),
            )
            .add_systems(
                Update,
                sync_transparent_star_collision_state
                    .after(reset_star_switch_state_on_play_world_spawn)
                    .after(MapSpawnSet),
            )
            .add_systems(
                Update,
                sync_transparent_star_visual_state
                    .after(reset_star_switch_state_on_play_world_spawn)
                    .after(prepare_transparent_star_visual_handles)
                    .after(BlockVisualSet),
            )
            .add_systems(
                Update,
                prepare_star_switch_visuals
                    .after(MapSpawnSet)
                    .after(BlockVisualSet),
            )
            .add_systems(
                Update,
                sync_star_switch_visual_state.after(prepare_star_switch_visuals),
            );
    }
}

fn attach_star_switch_sensors(
    mut commands: Commands,
    switches: Query<Entity, (With<StarSwitchTrigger>, Without<StarSwitchSensor>)>,
) {
    for entity in &switches {
        commands.entity(entity).insert((
            StarSwitchSensor,
            Sensor,
            CollisionEventsEnabled,
            Collider::rectangle(STAR_SWITCH_SENSOR_SIZE, STAR_SWITCH_SENSOR_SIZE),
            DebugRender::default().with_collider_color(STAR_SWITCH_SENSOR_COLOR),
        ));
    }
}

fn reset_star_switch_state_on_play_world_spawn(
    play_worlds: Query<(), Added<PlayWorld>>,
    mut contacts: ResMut<ActiveStarSwitchContacts>,
    mut state: ResMut<StarSwitchState>,
) {
    if play_worlds.iter().next().is_none() {
        return;
    }

    contacts.0.clear();

    if state.is_active() {
        state.reset();
    }
}

fn update_star_switch_contacts(
    mut collision_starts: MessageReader<CollisionStart>,
    mut collision_ends: MessageReader<CollisionEnd>,
    players: Query<(), With<PlayerBall>>,
    switches: Query<(), With<StarSwitchTrigger>>,
    mut contacts: ResMut<ActiveStarSwitchContacts>,
    mut state: ResMut<StarSwitchState>,
) {
    let mut contacts_changed = false;

    for event in collision_starts.read() {
        let contact = if players.contains(event.collider1) && switches.contains(event.collider2) {
            Some((event.collider1, event.collider2))
        } else if players.contains(event.collider2) && switches.contains(event.collider1) {
            Some((event.collider2, event.collider1))
        } else {
            None
        };

        let Some(contact) = contact else {
            continue;
        };

        contacts_changed |= contacts.0.insert(contact);
    }

    for event in collision_ends.read() {
        let contact = if players.contains(event.collider1) && switches.contains(event.collider2) {
            Some((event.collider1, event.collider2))
        } else if players.contains(event.collider2) && switches.contains(event.collider1) {
            Some((event.collider2, event.collider1))
        } else {
            None
        };

        let Some(contact) = contact else {
            continue;
        };

        contacts_changed |= contacts.0.remove(&contact);
    }

    if !contacts_changed {
        return;
    }

    sync_state_from_active_contacts(&contacts, &mut state);
}

fn prepare_transparent_star_visual_handles(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    stars: Query<Entity, (With<TransparentStar>, Without<TransparentStarVisualHandles>)>,
) {
    for entity in &stars {
        commands
            .entity(entity)
            .insert(TransparentStarVisualHandles {
                active: asset_server.load(STAR_ACTIVE_VISUAL_PATH),
                inactive: asset_server.load(STAR_INACTIVE_VISUAL_PATH),
            });
    }
}

fn sync_transparent_star_collision_state(
    mut commands: Commands,
    state: Res<StarSwitchState>,
    stars: Query<
        (Entity, Option<&ColliderDisabled>),
        (
            With<TransparentStar>,
            With<StarSensorCollider>,
            Without<CollectedStar>,
        ),
    >,
) {
    let is_active = state.is_active();

    for (entity, collider_disabled) in &stars {
        match (is_active, collider_disabled.is_some()) {
            // ON인데 ColliderDisabled가 있으면 제거.
            (true, true) => {
                commands.entity(entity).remove::<ColliderDisabled>();
            }

            // OFF인데 ColliderDisabled가 없으면 추가.
            (false, false) => {
                commands.entity(entity).insert(ColliderDisabled);
            }

            // 이미 올바른 상태.
            _ => {}
        }
    }
}

fn sync_transparent_star_visual_state(
    state: Res<StarSwitchState>,
    mut stars: Query<
        (&TransparentStarVisualHandles, &mut Sprite),
        (With<TransparentStar>, Without<CollectedStar>),
    >,
) {
    let target_is_active = state.is_active();

    for (visuals, mut sprite) in &mut stars {
        let target = match target_is_active {
            true => &visuals.active,
            false => &visuals.inactive,
        };

        if sprite.image != *target {
            sprite.image = target.clone();
        }
    }
}

fn prepare_star_switch_visuals(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    state: Res<StarSwitchState>,
    switches: Query<Entity, (With<StarSwitchTrigger>, Without<StarSwitchVisualAttached>)>,
) {
    let image = asset_server.load(if state.is_active() {
        STAR_ACTIVE_VISUAL_PATH
    } else {
        STAR_INACTIVE_VISUAL_PATH
    });

    for entity in &switches {
        commands.spawn((
            Name::new("Star switch visual"),
            StarSwitchVisual,
            Sprite {
                image: image.clone(),
                custom_size: Some(Vec2::splat(BLOCK_WORLD_SIZE)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, STAR_SWITCH_ICON_Z).with_scale(Vec3::new(
                STAR_SWITCH_ICON_SCALE,
                STAR_SWITCH_ICON_SCALE,
                1.0,
            )),
            ChildOf(entity),
        ));

        commands.entity(entity).insert(StarSwitchVisualAttached);
    }
}

fn sync_star_switch_visual_state(
    state: Res<StarSwitchState>,
    asset_server: Res<AssetServer>,
    mut visuals: Query<&mut Sprite, With<StarSwitchVisual>>,
) {
    if !state.is_changed() {
        return;
    }

    let target = asset_server.load(if state.is_active() {
        STAR_ACTIVE_VISUAL_PATH
    } else {
        STAR_INACTIVE_VISUAL_PATH
    });

    for mut sprite in &mut visuals {
        if sprite.image != target {
            sprite.image = target.clone();
        }
    }
}

fn sync_state_from_active_contacts(
    contacts: &ActiveStarSwitchContacts,
    state: &mut StarSwitchState,
) {
    let should_be_active = !contacts.0.is_empty();

    if state.is_active() != should_be_active {
        state.set_active(should_be_active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_switch_state_starts_inactive() {
        let state = StarSwitchState::default();

        assert!(!state.is_active());
    }

    #[test]
    fn star_switch_state_can_be_activated_and_reset() {
        let mut state = StarSwitchState::default();

        state.set_active(true);

        assert!(state.is_active());

        state.reset();

        assert!(!state.is_active());
    }

    #[test]
    fn star_switch_stays_active_until_last_contact_leaves() {
        let player_1 = Entity::from_raw_u32(1).unwrap();
        let player_2 = Entity::from_raw_u32(2).unwrap();

        let switch_1 = Entity::from_raw_u32(10).unwrap();
        let switch_2 = Entity::from_raw_u32(11).unwrap();

        let mut contacts = ActiveStarSwitchContacts::default();

        let mut state = StarSwitchState::default();

        contacts.0.insert((player_1, switch_1));
        contacts.0.insert((player_2, switch_2));

        sync_state_from_active_contacts(&contacts, &mut state);

        assert!(state.is_active());

        // 하나가 빠져도 다른 접촉이 남아 있으므로 ON 유지.
        contacts.0.remove(&(player_1, switch_1));

        sync_state_from_active_contacts(&contacts, &mut state);

        assert!(state.is_active());

        // 마지막 접촉까지 없어져야 OFF.
        contacts.0.remove(&(player_2, switch_2));

        sync_state_from_active_contacts(&contacts, &mut state);

        assert!(!state.is_active());
    }
}
