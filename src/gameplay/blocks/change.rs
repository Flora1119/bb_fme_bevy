use crate::gameplay::{
    BLOCK_WORLD_SIZE, BlockPhysicsBody, BlockVisualSet, MapSpawnSet, PHYSICS_HZ,
    PendingPlayInteractions, PhysicsInitializationSet, PlayInteraction, PlayInteractionCollectSet,
    PlayInteractionSet, PlayerBall, SolidBlock,
};
use avian2d::prelude::*;
use bevy::prelude::*;

pub const CHANGE_BLOCK_DURATION: f32 = 2.0;
pub const CHANGE_BLOCK_HALF_DURATION: f32 = CHANGE_BLOCK_DURATION * 0.5;
pub const CHANGE_BLOCK_TRIGGER_RADIUS: f32 = 0.2 * BLOCK_WORLD_SIZE;

//const CHANGE_BLOCK_END_VISUAL_PATH: &str = "sprites/whiteblock/wb_change_end.png";
const CHANGE_BLOCK_END_VISUAL_SIZE: Vec2 = Vec2::splat(0.9 * BLOCK_WORLD_SIZE);
const CHANGE_BLOCK_INNER_Z: f32 = 0.01;
const CHANGE_BLOCK_TRIGGER_COLOR: Color = Color::srgb(1.0, 0.35, 0.15);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChangeBlockPhase {
    #[default]
    Dormant,
    Changing,
    Solid,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ChangeBlock {
    phase: ChangeBlockPhase,
    elapsed: f32,
}

impl Default for ChangeBlock {
    fn default() -> Self {
        Self {
            phase: ChangeBlockPhase::Dormant,
            elapsed: 0.0,
        }
    }
}

impl ChangeBlock {
    pub const fn phase(&self) -> ChangeBlockPhase {
        self.phase
    }

    pub const fn elapsed(&self) -> f32 {
        self.elapsed
    }

    pub fn start(&mut self) {
        if self.phase != ChangeBlockPhase::Dormant {
            return;
        }

        self.phase = ChangeBlockPhase::Changing;
        self.elapsed = 0.0;
    }

    pub fn advance(&mut self, delta_seconds: f32) {
        if self.phase != ChangeBlockPhase::Changing {
            return;
        }

        self.elapsed += delta_seconds;

        if self.elapsed >= CHANGE_BLOCK_DURATION {
            self.elapsed = CHANGE_BLOCK_DURATION;
            self.phase = ChangeBlockPhase::Solid;
        }
    }

    pub fn reset(&mut self) {
        self.phase = ChangeBlockPhase::Dormant;
        self.elapsed = 0.0;
    }

    pub fn is_damaging(&self) -> bool {
        self.phase == ChangeBlockPhase::Changing && self.elapsed >= CHANGE_BLOCK_HALF_DURATION
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct ChangeBlockInnerColliderAttached;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeBlockInnerCollider {
    source: Entity,
}

impl ChangeBlockInnerCollider {
    const fn new(source: Entity) -> Self {
        Self { source }
    }

    pub const fn source(self) -> Entity {
        self.source
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeBlockTriggerSensor {
    source: Entity,
}

impl ChangeBlockTriggerSensor {
    pub const fn new(source: Entity) -> Self {
        Self { source }
    }

    pub const fn source(self) -> Entity {
        self.source
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct ChangeBlockVisualAttached;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct ChangeBlockInnerVisual {
    source: Entity,
}

impl ChangeBlockInnerVisual {
    const fn new(source: Entity) -> Self {
        Self { source }
    }

    const fn source(self) -> Entity {
        self.source
    }
}

pub struct ChangeBlockPlugin;

impl Plugin for ChangeBlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            attach_change_block_sensors
                .in_set(PhysicsInitializationSet)
                .after(MapSpawnSet),
        )
        .add_systems(
            PhysicsSchedule,
            collect_started_change_block_damage_interactions
                .in_set(PlayInteractionSet::Collect)
                .in_set(PlayInteractionCollectSet::Death)
                .after(advance_change_blocks),
        )
        .add_systems(
            PhysicsSchedule,
            (start_change_blocks_on_contact, advance_change_blocks)
                .chain()
                .after(PhysicsStepSystems::NarrowPhase)
                .before(PhysicsStepSystems::Solver),
        )
        .add_systems(Update, spawn_change_block_visuals.after(BlockVisualSet))
        .add_systems(
            Update,
            sync_change_block_visuals.after(spawn_change_block_visuals),
        )
        .add_systems(
            Update,
            spawn_change_block_inner_colliders.after(MapSpawnSet),
        )
        .add_systems(
            Update,
            sync_change_block_physics_state.after(spawn_change_block_inner_colliders),
        );
    }
}

fn attach_change_block_sensors(
    mut commands: Commands,
    blocks: Query<Entity, (With<ChangeBlock>, Without<BlockPhysicsBody>)>,
) {
    for entity in &blocks {
        commands
            .entity(entity)
            .insert((BlockPhysicsBody, RigidBody::Static));

        commands.spawn((
            Name::new("Collider: change block trigger"),
            ChangeBlockTriggerSensor::new(entity),
            Sensor,
            CollisionEventsEnabled,
            Collider::circle(CHANGE_BLOCK_TRIGGER_RADIUS),
            Transform::default(),
            DebugRender::default().with_collider_color(CHANGE_BLOCK_TRIGGER_COLOR),
            ChildOf(entity),
        ));
    }
}

fn start_change_blocks_on_contact(
    mut collision_starts: MessageReader<CollisionStart>,
    players: Query<(), With<PlayerBall>>,
    sensors: Query<&ChangeBlockTriggerSensor>,
    mut blocks: Query<&mut ChangeBlock>,
) {
    for event in collision_starts.read() {
        let sensor = if players.contains(event.collider1) {
            sensors.get(event.collider2).ok()
        } else if players.contains(event.collider2) {
            sensors.get(event.collider1).ok()
        } else {
            None
        };

        let Some(sensor) = sensor else {
            continue;
        };

        let Ok(mut block) = blocks.get_mut(sensor.source()) else {
            continue;
        };

        block.start();
    }
}

fn spawn_change_block_visuals(
    mut commands: Commands,
    blocks: Query<Entity, (With<ChangeBlock>, Without<ChangeBlockVisualAttached>)>,
) {
    for entity in &blocks {
        commands.spawn((
            Name::new("Change block inner visual"),
            ChangeBlockInnerVisual::new(entity),
            Sprite::from_color(
                Color::srgba(1.0, 0.0, 0.0, 0.0),
                CHANGE_BLOCK_END_VISUAL_SIZE,
            ),
            Transform::from_xyz(0.0, 0.0, CHANGE_BLOCK_INNER_Z),
            ChildOf(entity),
        ));

        commands.entity(entity).insert(ChangeBlockVisualAttached);
    }
}

fn change_block_color(block: &ChangeBlock) -> Color {
    match block.phase() {
        ChangeBlockPhase::Dormant => Color::srgba(1.0, 0.0, 0.0, 0.0),

        ChangeBlockPhase::Changing => {
            let elapsed = block.elapsed();

            if elapsed < CHANGE_BLOCK_HALF_DURATION {
                let alpha = (elapsed / CHANGE_BLOCK_HALF_DURATION).clamp(0.0, 1.0);

                Color::srgba(1.0, 0.0, 0.0, alpha)
            } else {
                let t = ((elapsed - CHANGE_BLOCK_HALF_DURATION) / CHANGE_BLOCK_HALF_DURATION)
                    .clamp(0.0, 1.0);

                const END_R: f32 = 41.0 / 255.0;
                const END_G: f32 = 170.0 / 255.0;
                const END_B: f32 = 220.0 / 255.0;

                let r = 1.0 + (END_R - 1.0) * t;
                let g = END_G * t;
                let b = END_B * t;

                Color::srgba(r, g, b, 1.0)
            }
        }

        ChangeBlockPhase::Solid => Color::srgba(41.0 / 255.0, 170.0 / 255.0, 220.0 / 255.0, 1.0),
    }
}
fn advance_change_blocks(mut blocks: Query<&mut ChangeBlock>) {
    let delta_seconds = (1.0 / PHYSICS_HZ) as f32;

    for mut block in &mut blocks {
        block.advance(delta_seconds);
    }
}

fn sync_change_block_visuals(
    blocks: Query<&ChangeBlock>,
    mut visuals: Query<(&ChangeBlockInnerVisual, &mut Sprite)>,
) {
    for (visual, mut sprite) in &mut visuals {
        let Ok(block) = blocks.get(visual.source()) else {
            continue;
        };
        sprite.image = Handle::default();
        sprite.color = change_block_color(block);
    }
}

fn spawn_change_block_inner_colliders(
    mut commands: Commands,
    blocks: Query<Entity, (With<ChangeBlock>, Without<ChangeBlockInnerColliderAttached>)>,
) {
    for entity in &blocks {
        commands.spawn((
            Name::new("Collider: change block inner"),
            ChangeBlockInnerCollider::new(entity),
            Collider::rectangle(BLOCK_WORLD_SIZE, BLOCK_WORLD_SIZE),
            // 후반 Damage 단계에서는 Sensor가 됩니다.
            Sensor,
            // 처음에는 존재하지만 비활성 상태.
            ColliderDisabled,
            CollisionEventsEnabled,
            Transform::default(),
            DebugRender::default().with_collider_color(Color::srgb(1.0, 0.2, 0.2)),
            ChildOf(entity),
        ));

        commands
            .entity(entity)
            .insert(ChangeBlockInnerColliderAttached);
    }
}

fn sync_change_block_physics_state(
    mut commands: Commands,

    blocks: Query<(&ChangeBlock, Option<&SolidBlock>)>,

    inner_colliders: Query<(
        Entity,
        &ChangeBlockInnerCollider,
        Option<&ColliderDisabled>,
        Option<&Sensor>,
    )>,

    trigger_sensors: Query<(Entity, &ChangeBlockTriggerSensor, Option<&ColliderDisabled>)>,
) {
    for (collider_entity, inner, collider_disabled, sensor) in &inner_colliders {
        let Ok((block, solid_block)) = blocks.get(inner.source()) else {
            continue;
        };

        match block.phase() {
            ChangeBlockPhase::Dormant => {
                // 내부 블록 없음.
                if collider_disabled.is_none() {
                    commands.entity(collider_entity).insert(ColliderDisabled);
                }

                // 나중에 다시 Damage 단계로 갈 수 있도록
                // Sensor 상태를 기본으로 유지.
                if sensor.is_none() {
                    commands.entity(collider_entity).insert(Sensor);
                }

                if solid_block.is_some() {
                    commands.entity(inner.source()).remove::<SolidBlock>();
                }
            }

            ChangeBlockPhase::Changing => {
                if block.is_damaging() {
                    // 1~2초:
                    // 1×1 Damage Sensor 활성화.
                    if collider_disabled.is_some() {
                        commands
                            .entity(collider_entity)
                            .remove::<ColliderDisabled>();
                    }

                    if sensor.is_none() {
                        commands.entity(collider_entity).insert(Sensor);
                    }
                } else {
                    // 0~1초:
                    // 아직 물리적으로 존재하지 않음.
                    if collider_disabled.is_none() {
                        commands.entity(collider_entity).insert(ColliderDisabled);
                    }

                    if sensor.is_none() {
                        commands.entity(collider_entity).insert(Sensor);
                    }
                }

                if solid_block.is_some() {
                    commands.entity(inner.source()).remove::<SolidBlock>();
                }
            }

            ChangeBlockPhase::Solid => {
                // 2초 완료:
                // Collider는 계속 켜져 있지만
                // Trigger가 아닌 실제 Solid가 됩니다.
                if collider_disabled.is_some() {
                    commands
                        .entity(collider_entity)
                        .remove::<ColliderDisabled>();
                }

                if sensor.is_some() {
                    commands.entity(collider_entity).remove::<Sensor>();
                }

                if solid_block.is_none() {
                    commands.entity(inner.source()).insert(SolidBlock);
                }
            }
        }
    }

    // 최종 Solid가 된 뒤에는 최초 작동용
    // 0.2 Trigger가 더 이상 필요하지 않습니다.
    for (trigger_entity, trigger, collider_disabled) in &trigger_sensors {
        let Ok((block, _)) = blocks.get(trigger.source()) else {
            continue;
        };

        match block.phase() {
            ChangeBlockPhase::Solid => {
                if collider_disabled.is_none() {
                    commands.entity(trigger_entity).insert(ColliderDisabled);
                }
            }

            _ => {
                if collider_disabled.is_some() {
                    commands.entity(trigger_entity).remove::<ColliderDisabled>();
                }
            }
        }
    }
}

fn collect_started_change_block_damage_interactions(
    mut collision_starts: MessageReader<CollisionStart>,

    players: Query<(), With<PlayerBall>>,

    damage_colliders: Query<&ChangeBlockInnerCollider, (With<Sensor>, Without<ColliderDisabled>)>,

    blocks: Query<&ChangeBlock>,

    mut pending: ResMut<PendingPlayInteractions>,
) {
    for event in collision_starts.read() {
        let damage_collider = if players.contains(event.collider1)
            && damage_colliders.contains(event.collider2)
        {
            event.collider2
        } else if players.contains(event.collider2) && damage_colliders.contains(event.collider1) {
            event.collider1
        } else {
            continue;
        };

        let Ok(inner) = damage_colliders.get(damage_collider) else {
            continue;
        };

        let Ok(block) = blocks.get(inner.source()) else {
            continue;
        };

        if !block.is_damaging() {
            continue;
        }

        pending.push(PlayInteraction::death(inner.source()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_block_starts_dormant() {
        let block = ChangeBlock::default();

        assert_eq!(block.phase(), ChangeBlockPhase::Dormant);

        assert_eq!(block.elapsed(), 0.0);
    }

    #[test]
    fn change_block_reaches_solid_after_two_seconds() {
        let mut block = ChangeBlock::default();

        block.start();

        assert_eq!(block.phase(), ChangeBlockPhase::Changing);

        block.advance(1.0);

        assert_eq!(block.phase(), ChangeBlockPhase::Changing);

        block.advance(1.0);

        assert_eq!(block.phase(), ChangeBlockPhase::Solid);

        assert_eq!(block.elapsed(), CHANGE_BLOCK_DURATION);
    }

    #[test]
    fn change_block_does_not_restart_while_changing() {
        let mut block = ChangeBlock::default();

        block.start();
        block.advance(0.75);

        block.start();

        assert_eq!(block.elapsed(), 0.75);
    }

    #[test]
    fn change_block_color_follows_original_animation() {
        let mut block = ChangeBlock::default();

        let dormant = change_block_color(&block).to_srgba();

        assert_eq!(dormant.alpha, 0.0);

        block.start();
        block.advance(0.5);

        let half_fade = change_block_color(&block).to_srgba();

        assert!((half_fade.red - 1.0).abs() < 0.001);
        assert!((half_fade.alpha - 0.5).abs() < 0.001);

        block.advance(0.5);

        let red = change_block_color(&block).to_srgba();

        assert!((red.red - 1.0).abs() < 0.001);
        assert!((red.green - 0.0).abs() < 0.001);
        assert!((red.blue - 0.0).abs() < 0.001);
        assert!((red.alpha - 1.0).abs() < 0.001);

        block.advance(1.0);

        let blue = change_block_color(&block).to_srgba();

        assert!((blue.red - 41.0 / 255.0).abs() < 0.001);
        assert!((blue.green - 170.0 / 255.0).abs() < 0.001);
        assert!((blue.blue - 220.0 / 255.0).abs() < 0.001);
        assert!((blue.alpha - 1.0).abs() < 0.001);
    }

    #[test]
    fn change_block_is_damaging_only_during_second_half() {
        let mut block = ChangeBlock::default();

        assert!(!block.is_damaging());

        block.start();

        block.advance(0.5);
        assert!(!block.is_damaging());

        block.advance(0.5);

        assert_eq!(block.phase(), ChangeBlockPhase::Changing);
        assert!(block.is_damaging());

        block.advance(0.5);
        assert!(block.is_damaging());

        block.advance(0.5);

        assert_eq!(block.phase(), ChangeBlockPhase::Solid);
        assert!(!block.is_damaging());
    }
}
