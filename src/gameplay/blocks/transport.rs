use bevy::prelude::Component;

/// 직진 이동 블록의 이동 방식입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StraightTransportMode {
    /// 한 번 밟으면 한 칸 이동합니다.
    OneStep,
    /// 한 번 밟으면 막힐 때까지 일정 간격으로 이동합니다.
    Continuous,
}

/// 공에게 직진 능력을 부여하는 StraightBlock과 달리,
/// 이 컴포넌트는 블록 자신을 이동시킵니다.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StraightTransportBlock {
    mode: StraightTransportMode,
}

impl StraightTransportBlock {
    pub const MOVE_DURATION_SECONDS: f32 = 0.3;
    pub const STEP_INTERVAL_SECONDS: f32 = 0.5;
    pub const MAX_CONTINUOUS_STEPS: u32 = 100;

    pub const fn one_step() -> Self {
        Self {
            mode: StraightTransportMode::OneStep,
        }
    }

    pub const fn continuous() -> Self {
        Self {
            mode: StraightTransportMode::Continuous,
        }
    }

    pub const fn mode(&self) -> StraightTransportMode {
        self.mode
    }
}

#[cfg(test)]
mod transport_spawn_tests {
    use super::*;
    use crate::{
        block::BlockAssetConfig,
        domain::{CardinalDirection, GridPosition, ValidatedMap},
        gameplay::{
            BlockFacing, CurrentGridPosition, GridIndex, MapSpawnPlugin, OriginGridPosition,
            SolidBlock, SpawnValidatedMap, StraightBlock,
        },
        map::MapDocument,
    };
    use bevy::prelude::*;

    const CONFIG: &str = include_str!("../../../assets/config/block_assets_config.json");

    const MINIMAL_MAP: &str = include_str!("../../../assets/maps/synthetic_minimal_map.json");

    fn map_with_transport(block_id: &str, direction: CardinalDirection) -> ValidatedMap {
        let config: BlockAssetConfig =
            serde_json::from_str(CONFIG).expect("config must deserialize");

        let mut json: serde_json::Value =
            serde_json::from_str(MINIMAL_MAP).expect("map must deserialize");

        let blocks = json["blocks"]
            .as_array_mut()
            .expect("blocks must be an array");

        let target = blocks
            .iter_mut()
            .find(|entry| entry["block"]["name"].as_str() == Some("b_normal"))
            .expect("fixture must contain a normal block");

        target["block"]["type"] = serde_json::json!("transport");
        target["block"]["name"] = serde_json::json!(block_id);
        target["block"]["dir"] = serde_json::json!(direction.index());

        let document: MapDocument = serde_json::from_value(json).expect("map must deserialize");

        ValidatedMap::from_document(&document, &config).expect("transport map must validate")
    }

    fn assert_spawned_mode(block_id: &str, expected_mode: StraightTransportMode) {
        for direction in CardinalDirection::ALL {
            let mut app = App::new();
            app.add_plugins(MapSpawnPlugin);

            app.world_mut()
                .write_message(SpawnValidatedMap(map_with_transport(block_id, direction)));

            app.update();

            let position = GridPosition::new(2, 0);
            let world = app.world();

            let entity = world
                .resource::<GridIndex>()
                .entity_at(position)
                .expect("transport block must be indexed");

            let transport = world
                .get::<StraightTransportBlock>(entity)
                .expect("transport component must exist");

            assert_eq!(transport.mode(), expected_mode);
            assert!(world.get::<SolidBlock>(entity).is_some());

            // 공을 직진시키는 기능 블록과 혼동하면 안 됩니다.
            assert!(world.get::<StraightBlock>(entity).is_none());

            assert_eq!(
                world.get::<BlockFacing>(entity),
                Some(&BlockFacing(direction)),
            );

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

    #[test]
    fn tp_st_o_spawns_as_one_step_solid_in_all_directions() {
        assert_spawned_mode("tp_st_o", StraightTransportMode::OneStep);
    }

    #[test]
    fn tp_st_a_spawns_as_continuous_solid_in_all_directions() {
        assert_spawned_mode("tp_st_a", StraightTransportMode::Continuous);
    }

    #[test]
    fn push_block_does_not_receive_self_movement_behavior() {
        let mut app = App::new();
        app.add_plugins(MapSpawnPlugin);

        app.world_mut()
            .write_message(SpawnValidatedMap(map_with_transport(
                "tp_push_o",
                CardinalDirection::Right,
            )));

        app.update();

        let world = app.world();
        let entity = world
            .resource::<GridIndex>()
            .entity_at(GridPosition::new(2, 0))
            .expect("push block must be indexed");

        assert!(world.get::<StraightTransportBlock>(entity).is_none());
    }
}
