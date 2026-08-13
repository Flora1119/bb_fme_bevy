use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub type ExtraFields = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapDocument {
    pub map_name: String,
    pub author: String,
    pub map_settings: MapSettings,
    pub blocks: Vec<MapBlockEntry>,

    #[serde(default)]
    pub block_options: Option<Vec<MapBlockOption>>,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapSettings {
    pub time_limit: f32,
    pub show_time_ranking: bool,
    pub star_count: i32,
    pub size: MapSize,
    pub tp1_exit: MapPosition,
    pub tp2_exit: MapPosition,
    pub portal1_positions: PortalPositions,
    pub portal2_positions: PortalPositions,
    pub sw_el: bool,
    pub sw_b1: bool,
    pub sw_b2: bool,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapSize {
    pub width: i32,
    pub height: i32,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapPosition {
    pub x: i32,
    pub y: i32,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortalPositions {
    pub a_px: i32,
    pub a_py: i32,
    pub b_px: i32,
    pub b_py: i32,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapBlockEntry {
    pub x: i32,
    pub y: i32,
    pub block: MapBlock,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapBlock {
    pub r#type: String,
    pub name: String,
    pub dir: i32,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapBlockOption {
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub options: Vec<MapBlockOptionValue>,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapBlockOptionValue {
    pub value_name: String,
    pub value: f32,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}
