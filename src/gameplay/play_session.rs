use super::{CollectedStar, CollectibleStar};
use avian2d::prelude::*;
use bevy::prelude::*;

pub struct PlaySessionPlugin;

impl Plugin for PlaySessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlaySession>()
            .init_resource::<PendingPlayInteractions>()
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
    Movement { source: Entity },

    Collection { source: Entity },

    Switch { source: Entity },
}

impl PlayInteraction {
    pub const fn death(source: Entity) -> Self {
        Self::Death { source }
    }

    pub const fn movement(source: Entity) -> Self {
        Self::Movement { source }
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
            | Self::Movement { source }
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

    fn sort_key(self) -> (u8, u64) {
        (self.priority(), self.source().to_bits())
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
    collectible_stars: Query<(), (With<CollectibleStar>, Without<CollectedStar>)>,
) {
    let mut interactions = std::mem::take(&mut pending.interactions);

    normalize_interactions(&mut interactions);

    if !session.is_playing() {
        return;
    }

    for interaction in interactions {
        match interaction {
            PlayInteraction::Death { .. } => {
                session.mark_dead();

                // Death는 가장 높은 우선순위입니다.
                // 같은 틱의 나머지 게임 규칙은 실행하지 않습니다.
                break;
            }

            PlayInteraction::Movement { .. } => {
                // Phase 4 후속 작업에서 포탈/강제 이동 규칙을 연결합니다.
            }

            PlayInteraction::Collection { source } => {
                if !collectible_stars.contains(source) {
                    continue;
                }

                session.collect_star();

                commands.entity(source).insert((
                    CollectedStar,
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
        let movement = PlayInteraction::movement(entity(3));
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
}
