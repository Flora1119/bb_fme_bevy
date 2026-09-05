use super::{
    AbilityInventory, AbilityItem, AbilityItemEffect, ActivePlayWorld, CollectedAbilityItem,
    CollectedStar, CollectibleStar, CurrentGridPosition, PlayWorld, PlayerAbility, PlayerBall,
    PlayerGravityState, TeleportCheckpoint,
};
use avian2d::prelude::*;
use bevy::prelude::*;

pub struct PlaySessionPlugin;

impl Plugin for PlaySessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlaySession>()
            .init_resource::<PendingPlayInteractions>()
            // 여러 interaction collector가 같은 물리 틱에
            // PendingPlayInteractions에 독립적으로 push할 수 있습니다.
            //
            // 실제 처리 전에 normalize_interactions()가
            // priority / source / actor 기준으로 결정적 정렬하므로
            // collector들의 실행 순서는 게임 결과에 영향을 주지 않습니다.
            .allow_ambiguous_resource::<PendingPlayInteractions>()
            .add_message::<ResolvedMovementInteraction>()
            .configure_sets(
                PhysicsSchedule,
                (
                    PlaySessionSet::AdvanceTime
                        .after(PhysicsStepSystems::First)
                        .before(PhysicsStepSystems::BroadPhase),
                    PlayInteractionSet::Collect
                        .after(PhysicsStepSystems::NarrowPhase)
                        .before(PhysicsStepSystems::Solver),
                    PlayInteractionSet::Resolve
                        .after(PhysicsStepSystems::Solver)
                        .before(PhysicsStepSystems::Sleeping),
                ),
            )
            .configure_sets(
                PhysicsSchedule,
                (
                    PlayInteractionCollectSet::Death,
                    PlayInteractionCollectSet::BoundaryDeath,
                    PlayInteractionCollectSet::Movement,
                    PlayInteractionCollectSet::Collection,
                )
                    .chain(),
            )
            .add_systems(
                PhysicsSchedule,
                advance_play_session_time.in_set(PlaySessionSet::AdvanceTime),
            )
            .add_systems(
                PhysicsSchedule,
                resolve_pending_play_interactions.in_set(PlayInteractionSet::Resolve),
            );
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaySessionSet {
    AdvanceTime,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayInteractionSet {
    Collect,
    Resolve,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayInteractionCollectSet {
    Death,
    BoundaryDeath,
    Movement,
    Collection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaySessionState {
    Playing,
    Dead,
    Cleared,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct PlaySession {
    state: PlaySessionState,
    collected_stars: u32,
    elapsed_seconds: f32,
}

impl Default for PlaySession {
    fn default() -> Self {
        Self {
            state: PlaySessionState::Playing,
            collected_stars: 0,
            elapsed_seconds: 0.0,
        }
    }
}

impl PlaySession {
    pub const fn state(&self) -> PlaySessionState {
        self.state
    }

    pub const fn is_playing(&self) -> bool {
        matches!(self.state, PlaySessionState::Playing)
    }

    pub const fn collected_stars(&self) -> u32 {
        self.collected_stars
    }

    pub const fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    pub fn mark_dead(&mut self) {
        if self.is_playing() {
            self.state = PlaySessionState::Dead;
        }
    }

    pub fn mark_cleared(&mut self) {
        if self.is_playing() {
            self.state = PlaySessionState::Cleared;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn collect_star(&mut self) {
        if self.is_playing() {
            self.collected_stars = self.collected_stars.saturating_add(1);
        }
    }

    fn advance_time(&mut self, delta_seconds: f32) {
        if self.is_playing() {
            self.elapsed_seconds += delta_seconds.max(0.0);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayInteraction {
    Death { source: Entity },

    // 포탈이나 강제 이동 계열이 나중에 이 우선순위를 사용합니다.
    Movement { source: Entity, actor: Entity },

    Collection { source: Entity },

    Switch { source: Entity },
}

impl PlayInteraction {
    pub const fn death(source: Entity) -> Self {
        Self::Death { source }
    }

    pub const fn movement(source: Entity, actor: Entity) -> Self {
        Self::Movement { source, actor }
    }

    pub const fn collection(source: Entity) -> Self {
        Self::Collection { source }
    }

    pub const fn switch(source: Entity) -> Self {
        Self::Switch { source }
    }

    pub const fn source(self) -> Entity {
        match self {
            Self::Death { source }
            | Self::Movement { source, .. }
            | Self::Collection { source }
            | Self::Switch { source } => source,
        }
    }

    const fn priority(self) -> u8 {
        match self {
            Self::Death { .. } => 0,
            Self::Movement { .. } => 1,
            Self::Collection { .. } => 2,
            Self::Switch { .. } => 3,
        }
    }

    fn sort_key(self) -> (u8, u64, u64) {
        let actor = match self {
            Self::Movement { actor, .. } => actor.to_bits(),
            _ => 0,
        };

        (self.priority(), self.source().to_bits(), actor)
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMovementInteraction {
    source: Entity,
    actor: Entity,
}

impl ResolvedMovementInteraction {
    pub const fn new(source: Entity, actor: Entity) -> Self {
        Self { source, actor }
    }

    pub const fn source(self) -> Entity {
        self.source
    }

    pub const fn actor(self) -> Entity {
        self.actor
    }
}

#[derive(Resource, Debug, Default)]
pub struct PendingPlayInteractions {
    interactions: Vec<PlayInteraction>,
}

impl PendingPlayInteractions {
    pub fn push(&mut self, interaction: PlayInteraction) {
        self.interactions.push(interaction);
    }
}

fn advance_play_session_time(time: Res<Time<Physics>>, mut session: ResMut<PlaySession>) {
    session.advance_time(time.delta_secs());
}

fn resolve_pending_play_interactions(
    mut commands: Commands,
    mut pending: ResMut<PendingPlayInteractions>,
    mut session: ResMut<PlaySession>,
    mut resolved_movements: MessageWriter<ResolvedMovementInteraction>,
    active_play_world: Option<Res<ActivePlayWorld>>,
    play_worlds: Query<&PlayWorld>,
    collectible_stars: Query<(), (With<CollectibleStar>, Without<CollectedStar>)>,
    ability_items: Query<(&AbilityItem, &CurrentGridPosition), Without<CollectedAbilityItem>>,
    mut gravity_state: Option<ResMut<PlayerGravityState>>,
    mut player_gravity_scales: Query<&mut GravityScale, With<PlayerBall>>,
    mut ability_inventory: Option<ResMut<AbilityInventory>>,
    mut teleport_checkpoint: Option<ResMut<TeleportCheckpoint>>,
) {
    let mut interactions = std::mem::take(&mut pending.interactions);

    normalize_interactions(&mut interactions);

    if !session.is_playing() {
        return;
    }

    let required_stars = active_play_world
        .as_ref()
        .and_then(|active_play_world| active_play_world.root())
        .and_then(|root| play_worlds.get(root).ok())
        .map(|play_world| play_world.definition().settings.required_stars);

    for interaction in interactions {
        match interaction {
            PlayInteraction::Death { .. } => {
                session.mark_dead();

                // Death는 가장 높은 우선순위입니다.
                // 같은 틱의 나머지 게임 규칙은 실행하지 않습니다.
                break;
            }

            PlayInteraction::Movement { source, actor } => {
                resolved_movements.write(ResolvedMovementInteraction::new(source, actor));

                // 강제 이동이 결정되면 같은 물리 틱에서
                // 이전 위치에서 발생한 Collection / Switch는 처리하지 않습니다.
                break;
            }

            PlayInteraction::Collection { source } => {
                // 1. 별 획득
                if collectible_stars.contains(source) {
                    session.collect_star();

                    commands.entity(source).insert((
                        CollectedStar,
                        Visibility::Hidden,
                        ColliderDisabled,
                    ));

                    if required_stars
                        .is_some_and(|required_stars| session.collected_stars() >= required_stars)
                    {
                        session.mark_cleared();

                        break;
                    }

                    continue;
                }

                // 2. 능력 아이템 획득
                let Ok((item, item_position)) = ability_items.get(source) else {
                    continue;
                };

                match item.effect() {
                    AbilityItemEffect::Queue(ability) => {
                        if ability == PlayerAbility::Teleport {
                            if let Some(checkpoint) = teleport_checkpoint.as_mut() {
                                checkpoint.activate(item_position.0);
                            }
                        }

                        let Some(inventory) = ability_inventory.as_mut() else {
                            continue;
                        };

                        inventory.enqueue(ability);
                    }

                    AbilityItemEffect::AdjustGravityScale(adjustment) => {
                        let Some(state) = gravity_state.as_mut() else {
                            continue;
                        };

                        state.adjust_scale(adjustment);

                        for mut gravity_scale in &mut player_gravity_scales {
                            gravity_scale.0 = state.scale();
                        }
                    }

                    AbilityItemEffect::SetInvisible(_) => {
                        // 5-B-1.8.3에서 구현.
                        continue;
                    }
                }

                commands.entity(source).insert((
                    CollectedAbilityItem,
                    Visibility::Hidden,
                    ColliderDisabled,
                ));
            }

            PlayInteraction::Switch { .. } => {
                // Phase 5 이후 스위치 규칙이 이 위치에 연결됩니다.
            }
        }
    }
}

fn normalize_interactions(interactions: &mut Vec<PlayInteraction>) {
    interactions.sort_unstable_by_key(|interaction| interaction.sort_key());

    // 하나의 Entity가 같은 물리 틱에 동일한 상호작용을
    // 중복 발생시켜도 한 번만 처리합니다.
    interactions.dedup_by_key(|interaction| interaction.sort_key());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(index: u32) -> Entity {
        Entity::from_raw_u32(index).expect("test entity index must be valid")
    }

    #[test]
    fn default_session_starts_in_playing_state() {
        let session = PlaySession::default();

        assert_eq!(session.state(), PlaySessionState::Playing);
        assert_eq!(session.collected_stars(), 0);
        assert_eq!(session.elapsed_seconds(), 0.0);
    }

    #[test]
    fn interaction_priority_is_independent_of_arrival_order() {
        let death = PlayInteraction::death(entity(4));
        let movement = PlayInteraction::movement(entity(3), entity(30));
        let collection = PlayInteraction::collection(entity(2));
        let switch = PlayInteraction::switch(entity(1));

        let expected = vec![death, movement, collection, switch];

        let mut first = vec![switch, collection, movement, death];

        let mut second = vec![movement, death, switch, collection];

        normalize_interactions(&mut first);
        normalize_interactions(&mut second);

        assert_eq!(first, expected);
        assert_eq!(second, expected);
    }

    #[test]
    fn duplicate_interactions_from_the_same_source_are_removed() {
        let star = entity(1);

        let mut interactions = vec![
            PlayInteraction::collection(star),
            PlayInteraction::collection(star),
            PlayInteraction::collection(star),
        ];

        normalize_interactions(&mut interactions);

        assert_eq!(interactions, vec![PlayInteraction::collection(star)]);
    }

    #[test]
    fn death_wins_over_collection_in_the_same_batch() {
        let star = entity(1);
        let spike = entity(2);

        for mut interactions in [
            vec![
                PlayInteraction::collection(star),
                PlayInteraction::death(spike),
            ],
            vec![
                PlayInteraction::death(spike),
                PlayInteraction::collection(star),
            ],
        ] {
            let mut session = PlaySession::default();

            normalize_interactions(&mut interactions);

            for interaction in interactions {
                match interaction {
                    PlayInteraction::Death { .. } => {
                        session.mark_dead();
                        break;
                    }
                    PlayInteraction::Collection { .. } => {
                        session.collect_star();
                    }
                    PlayInteraction::Movement { .. } | PlayInteraction::Switch { .. } => {}
                }
            }

            assert_eq!(session.state(), PlaySessionState::Dead);
            assert_eq!(session.collected_stars(), 0);
        }
    }

    #[test]
    fn elapsed_time_only_advances_while_playing() {
        let mut session = PlaySession::default();

        session.advance_time(0.25);

        assert!((session.elapsed_seconds() - 0.25).abs() <= f32::EPSILON);

        session.mark_dead();
        session.advance_time(1.0);

        assert!((session.elapsed_seconds() - 0.25).abs() <= f32::EPSILON);
    }

    #[test]
    fn collecting_teleport_item_activates_checkpoint_at_item_position() {
        use crate::domain::GridPosition;

        let mut app = App::new();

        app.insert_resource(PlaySession::default())
            .insert_resource(PendingPlayInteractions::default())
            .insert_resource(AbilityInventory::default())
            .insert_resource(TeleportCheckpoint::default())
            .add_message::<ResolvedMovementInteraction>()
            .add_systems(Update, resolve_pending_play_interactions);

        let item_position = GridPosition::new(7, -3);

        let item = app
            .world_mut()
            .spawn((
                AbilityItem::new(AbilityItemEffect::Queue(PlayerAbility::Teleport)),
                CurrentGridPosition(item_position),
            ))
            .id();

        app.world_mut()
            .resource_mut::<PendingPlayInteractions>()
            .push(PlayInteraction::collection(item));

        app.update();

        assert_eq!(
            app.world().resource::<TeleportCheckpoint>().position(),
            Some(item_position),
        );

        assert_eq!(
            app.world().resource::<AbilityInventory>().current(),
            Some(PlayerAbility::Teleport),
        );
    }
}
