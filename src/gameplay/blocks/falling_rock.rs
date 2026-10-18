use crate::domain::ValidatedBlockOption;
use bevy::prelude::Component;

/// 낙석을 주기적으로 만드는 생성기입니다.
///
/// 생성기 자체는 보이지 않으며, 고체 충돌체를 갖지 않습니다.
/// 실제로 떨어지는 낙석은 별도의 엔티티로 생성합니다.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct FallingRockSpawner {
    delay_seconds: f32,
    interval_seconds: f32,
}

impl FallingRockSpawner {
    /// 검증 및 기본값 적용이 끝난 맵 옵션으로 생성합니다.
    pub fn from_options(options: &[ValidatedBlockOption]) -> Self {
        let value = |name: &str| {
            options
                .iter()
                .find(|option| option.name == name)
                .unwrap_or_else(|| panic!("validated fb_fr options must contain {name}"))
                .value
        };

        Self {
            delay_seconds: value("Delay"),
            interval_seconds: value("Interval"),
        }
    }

    pub const fn delay_seconds(&self) -> f32 {
        self.delay_seconds
    }

    pub const fn interval_seconds(&self) -> f32 {
        self.interval_seconds
    }
}

#[cfg(test)]
mod falling_rock_spawn_tests {
    use super::*;
    use crate::{
        block::BlockAssetConfig,
        domain::{GridPosition, ValidatedMap},
        gameplay::{
            CurrentGridPosition, GridIndex, MapSpawnPlugin, OriginGridPosition, SolidBlock,
            SpawnValidatedMap,
        },
        map::MapDocument,
    };
    use bevy::prelude::*;

    fn spawner_map() -> ValidatedMap {
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

        target["block"]["type"] = serde_json::json!("funcblock");
        target["block"]["name"] = serde_json::json!("fb_fr");

        let document: MapDocument = serde_json::from_value(json).unwrap();

        ValidatedMap::from_document(&document, &config).unwrap()
    }

    fn spawn(map: ValidatedMap) -> (App, Entity) {
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
    fn falling_rock_spawner_uses_config_defaults_and_is_hidden_non_solid() {
        let (app, entity) = spawn(spawner_map());
        let world = app.world();

        let spawner = world.get::<FallingRockSpawner>(entity).unwrap();

        assert_eq!(spawner.delay_seconds(), 0.5);
        assert_eq!(spawner.interval_seconds(), 1.0);

        assert_eq!(world.get::<Visibility>(entity), Some(&Visibility::Hidden));
        assert!(world.get::<SolidBlock>(entity).is_none());

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

    #[test]
    fn falling_rock_spawner_reads_options_by_name_instead_of_order() {
        let mut map = spawner_map();

        let block = map
            .blocks
            .iter_mut()
            .find(|block| block.id.as_str() == "fb_fr")
            .unwrap();

        // 순서를 뒤집어도 이름으로 올바른 값을 읽어야 합니다.
        block.options = vec![
            ValidatedBlockOption {
                name: "Interval".to_owned(),
                value: 2.5,
            },
            ValidatedBlockOption {
                name: "Delay".to_owned(),
                value: 0.0,
            },
        ];

        let (app, entity) = spawn(map);
        let spawner = app.world().get::<FallingRockSpawner>(entity).unwrap();

        assert_eq!(spawner.delay_seconds(), 0.0);
        assert_eq!(spawner.interval_seconds(), 2.5);
    }
}
