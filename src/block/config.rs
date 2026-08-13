use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const EXPECTED_BLOCK_CATEGORY_COUNT: usize = 9;
pub const EXPECTED_BLOCK_ID_COUNT: usize = 82;

type ExtraFields = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlockId(String);

impl BlockId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BlockId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for BlockId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl AsRef<str> for BlockId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for BlockId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockCategory {
    Item,
    Block,
    Spike,
    Funcblock,
    Switch,
    Whiteblock,
    Transport,
    Laser,
    Obstacle,
}

impl BlockCategory {
    pub const ALL: [Self; EXPECTED_BLOCK_CATEGORY_COUNT] = [
        Self::Item,
        Self::Block,
        Self::Spike,
        Self::Funcblock,
        Self::Switch,
        Self::Whiteblock,
        Self::Transport,
        Self::Laser,
        Self::Obstacle,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Block => "block",
            Self::Spike => "spike",
            Self::Funcblock => "funcblock",
            Self::Switch => "switch",
            Self::Whiteblock => "whiteblock",
            Self::Transport => "transport",
            Self::Laser => "laser",
            Self::Obstacle => "obstacle",
        }
    }
}

impl fmt::Display for BlockCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAssetConfig {
    pub block_groups: BTreeMap<BlockCategory, Vec<BlockId>>,
    pub block_type_settings: BTreeMap<BlockCategory, BlockTypeSetting>,
    pub block_exceptions: Vec<BlockId>,
    pub block_rotatable: Vec<BlockId>,
    pub gear_passable_blocks: Vec<BlockId>,
    pub option_blocks: Vec<BlockId>,
    pub block_options: BTreeMap<BlockId, Vec<BlockOptionDefinition>>,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockTypeSetting {
    pub rotatable: bool,
    pub colorable: bool,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockOptionDefinition {
    pub value_name: String,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,

    #[serde(default, flatten)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationErrors {
    problems: Vec<String>,
}

impl ConfigValidationErrors {
    pub fn problems(&self) -> &[String] {
        &self.problems
    }
}

impl fmt::Display for ConfigValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.problems.join("\n"))
    }
}

impl Error for ConfigValidationErrors {}

impl BlockAssetConfig {
    pub fn block_id_count(&self) -> usize {
        self.block_groups.values().map(Vec::len).sum()
    }

    pub fn all_block_ids(&self) -> impl Iterator<Item = &BlockId> {
        self.block_groups.values().flatten()
    }

    pub fn blocks_in(&self, category: BlockCategory) -> Option<&[BlockId]> {
        self.block_groups.get(&category).map(Vec::as_slice)
    }

    pub fn validate(&self) -> Result<(), ConfigValidationErrors> {
        let mut problems = Vec::new();

        self.validate_categories(&mut problems);

        if self.block_id_count() != EXPECTED_BLOCK_ID_COUNT {
            problems.push(format!(
                "expected {EXPECTED_BLOCK_ID_COUNT} block IDs, found {}",
                self.block_id_count()
            ));
        }

        let mut known_ids = BTreeSet::new();

        for block_id in self.all_block_ids() {
            if !known_ids.insert(block_id.clone()) {
                problems.push(format!("duplicate block ID: {block_id}"));
            }
        }

        validate_references(
            &mut problems,
            &known_ids,
            "blockRotatable",
            &self.block_rotatable,
        );

        validate_references(
            &mut problems,
            &known_ids,
            "gearPassableBlocks",
            &self.gear_passable_blocks,
        );

        validate_references(
            &mut problems,
            &known_ids,
            "optionBlocks",
            &self.option_blocks,
        );

        self.validate_option_definitions(&mut problems);

        if problems.is_empty() {
            Ok(())
        } else {
            Err(ConfigValidationErrors { problems })
        }
    }

    fn validate_categories(&self, problems: &mut Vec<String>) {
        if self.block_groups.len() != EXPECTED_BLOCK_CATEGORY_COUNT {
            problems.push(format!(
                "expected {EXPECTED_BLOCK_CATEGORY_COUNT} block groups, found {}",
                self.block_groups.len()
            ));
        }

        if self.block_type_settings.len() != EXPECTED_BLOCK_CATEGORY_COUNT {
            problems.push(format!(
                "expected {EXPECTED_BLOCK_CATEGORY_COUNT} block type settings, found {}",
                self.block_type_settings.len()
            ));
        }

        for category in BlockCategory::ALL {
            if !self.block_groups.contains_key(&category) {
                problems.push(format!("missing block group: {category}"));
            }

            if !self.block_type_settings.contains_key(&category) {
                problems.push(format!("missing block type setting: {category}"));
            }
        }
    }

    fn validate_option_definitions(&self, problems: &mut Vec<String>) {
        for block_id in &self.option_blocks {
            if !self.block_options.contains_key(block_id) {
                problems.push(format!("option block {block_id} has no option definition"));
            }
        }

        for block_id in self.block_options.keys() {
            if !self.option_blocks.contains(block_id) {
                problems.push(format!(
                    "option definition exists for undeclared block {block_id}"
                ));
            }
        }

        for (block_id, options) in &self.block_options {
            let mut option_names = BTreeSet::new();

            for option in options {
                if !option_names.insert(option.value_name.as_str()) {
                    problems.push(format!(
                        "duplicate option {} on block {block_id}",
                        option.value_name
                    ));
                }

                if option.min > option.max {
                    problems.push(format!(
                        "option {} on block {block_id} has min {} greater than max {}",
                        option.value_name, option.min, option.max
                    ));
                }

                if option.default_value < option.min || option.default_value > option.max {
                    problems.push(format!(
                        "option {} on block {block_id} has default {} outside {}..={}",
                        option.value_name, option.default_value, option.min, option.max
                    ));
                }
            }
        }
    }
}

fn validate_references(
    problems: &mut Vec<String>,
    known_ids: &BTreeSet<BlockId>,
    list_name: &str,
    block_ids: &[BlockId],
) {
    let mut seen = BTreeSet::new();

    for block_id in block_ids {
        if !known_ids.contains(block_id) {
            problems.push(format!("{list_name} references unknown block {block_id}"));
        }

        if !seen.insert(block_id) {
            problems.push(format!("{list_name} contains duplicate block {block_id}"));
        }
    }
}
