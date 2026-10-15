use bevy::prelude::Component;

/// 대상 블록을 어느 쪽으로 옮기는지 나타냅니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushPullAction {
    Push,
    Pull,
}

/// 대상 블록을 얼마나 옮기는지 나타냅니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushPullReach {
    OneCell,
    ToLimit,
}

/// 자신이 아닌, 화살표 방향에서 발견한 블록을 이동시킵니다.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushPullBlock {
    action: PushPullAction,
    reach: PushPullReach,
}

impl PushPullBlock {
    pub const fn new(action: PushPullAction, reach: PushPullReach) -> Self {
        Self { action, reach }
    }

    pub const fn action(&self) -> PushPullAction {
        self.action
    }

    pub const fn reach(&self) -> PushPullReach {
        self.reach
    }
}

#[cfg(test)]
mod push_pull_spawn_tests {
    use super::*;
    use crate::{
        block::BlockAssetConfig,
        domain::{CardinalDirection, GridPosition, ValidatedMap},
        gameplay::{
            BlockFacing, CurrentGridPosition, GridIndex, MapSpawnPlugin, OriginGridPosition,
            SolidBlock, SpawnValidatedMap, StraightTransportBlock,
        },
        map::MapDocument,
    };
    use bevy::prelude::*;

    fn spawn_block(block_id: &str, direction: CardinalDirection) -> (App, Entity) {
        let config: BlockAssetConfig = serde_json::from_str(include_str!(
            "../../../assets/config/block_assets_config.json"
        ))
        .unwrap();

        let mut json: serde_json::Value = serde_json::from_str(include_str!(
            "../../../assets/maps/synthetic_minimal_map.json"
        ))
        .unwrap();

        let target = json["blocks"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["block"]["name"].as_str() == Some("b_normal"))
            .unwrap();

        target["block"]["type"] = serde_json::json!("transport");
        target["block"]["name"] = serde_json::json!(block_id);
        target["block"]["dir"] = serde_json::json!(direction.index());

        let document: MapDocument = serde_json::from_value(json).unwrap();
        let map = ValidatedMap::from_document(&document, &config).unwrap();

        let mut app = App::new();
        app.add_plugins(MapSpawnPlugin);
        app.world_mut().write_message(SpawnValidatedMap(map));
        app.update();

        let entity = app
            .world()
            .resource::<GridIndex>()
            .entity_at(GridPosition::new(2, 0))
            .unwrap();

        (app, entity)
    }

    #[test]
    fn push_pull_variants_spawn_with_correct_behavior_in_all_directions() {
        let cases = [
            ("tp_push_o", PushPullAction::Push, PushPullReach::OneCell),
            ("tp_push_a", PushPullAction::Push, PushPullReach::ToLimit),
            ("tp_pull_o", PushPullAction::Pull, PushPullReach::OneCell),
            ("tp_pull_a", PushPullAction::Pull, PushPullReach::ToLimit),
        ];

        for (block_id, action, reach) in cases {
            for direction in CardinalDirection::ALL {
                let (app, entity) = spawn_block(block_id, direction);
                let world = app.world();
                let block = world.get::<PushPullBlock>(entity).unwrap();

                assert_eq!(block.action(), action, "{block_id}");
                assert_eq!(block.reach(), reach, "{block_id}");
                assert!(world.get::<SolidBlock>(entity).is_some());

                // 자신이 이동하는 블록의 동작이 붙으면 안 됩니다.
                assert!(world.get::<StraightTransportBlock>(entity).is_none());

                assert_eq!(
                    world.get::<BlockFacing>(entity),
                    Some(&BlockFacing(direction)),
                );

                let position = GridPosition::new(2, 0);

                assert_eq!(
                    world.get::<CurrentGridPosition>(entity),
                    Some(&CurrentGridPosition(position)),
                );
                assert_eq!(
                    world.get::<OriginGridPosition>(entity),
                    Some(&OriginGridPosition(position)),
                );
            }
        }
    }

    #[test]
    fn straight_transports_do_not_receive_push_pull_behavior() {
        for block_id in ["tp_st_o", "tp_st_a"] {
            let (app, entity) = spawn_block(block_id, CardinalDirection::Right);
            let world = app.world();

            assert!(world.get::<StraightTransportBlock>(entity).is_some());
            assert!(world.get::<PushPullBlock>(entity).is_none());
        }
    }
}
