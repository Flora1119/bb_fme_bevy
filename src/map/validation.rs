use serde_json::error::Category;

use super::{MapBlockEntry, MapBlockOption, MapDocument};
use crate::block::{BlockAssetConfig, BlockCategory};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const MIN_MAP_WIDTH: i32 = 25;
pub const MIN_MAP_HEIGHT: i32 = 15;
pub const MAX_MAP_WIDTH: i32 = 45;
pub const MAX_MAP_HEIGHT: i32 = 45;

const BALL_ID: &str = "ball";
const STAR_ID: &str = "star";
const EMPTY_STAR_ID: &str = "star_empty";
const JUMP_STAR_ID: &str = "star_jump";
const STAR_SWITCH_ID: &str = "wb_star_sw";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapValidationErrors {
    problems: Vec<String>,
}

impl MapValidationErrors {
    pub fn problems(&self) -> &[String] {
        &self.problems
    }
}

impl fmt::Display for MapValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.problems.join("\n"))
    }
}

impl Error for MapValidationErrors {}

pub fn validate_map_document(
    document: &MapDocument,
    config: &BlockAssetConfig,
) -> Result<(), MapValidationErrors> {
    let mut problems = Vec::new();

    validate_config(config, &mut problems);
    validate_metadata(document, &mut problems);

    let blocks_by_position = validate_blocks(document, config, &mut problems);

    validate_block_options(
        document.block_options.as_deref(),
        config,
        &blocks_by_position,
        &mut problems,
    );

    if problems.is_empty() {
        Ok(())
    } else {
        Err(MapValidationErrors { problems })
    }
}

fn validate_config(config: &BlockAssetConfig, problems: &mut Vec<String>) {
    if let Err(errors) = config.validate() {
        for problem in errors.problems() {
            problems.push(format!("block config is invalid: {problem}"));
        }
    }
}

fn validate_metadata(document: &MapDocument, problems: &mut Vec<String>) {
    if document.map_name.trim().is_empty() {
        problems.push("map name must not be empty".to_owned());
    }

    if document.author.trim().is_empty() {
        problems.push("map author must not be empty".to_owned());
    }

    let size = &document.map_settings.size;

    if !(MIN_MAP_WIDTH..=MAX_MAP_WIDTH).contains(&size.width) {
        problems.push(format!(
            "map width {} is outside {MIN_MAP_WIDTH}..={MAX_MAP_WIDTH}",
            size.width
        ));
    }

    if !(MIN_MAP_HEIGHT..=MAX_MAP_HEIGHT).contains(&size.height) {
        problems.push(format!(
            "map height {} is outside {MIN_MAP_HEIGHT}..={MAX_MAP_HEIGHT}",
            size.height
        ));
    }

    let time_limit = document.map_settings.time_limit;

    if !time_limit.is_finite() || time_limit < 0.0 {
        problems.push(format!(
            "time limit must be a finite non-negative number, found {time_limit}"
        ));
    }

    if document.map_settings.star_count <= 0 {
        problems.push(format!(
            "star count must be positive, found {}",
            document.map_settings.star_count
        ));
    }
}

fn validate_blocks<'a>(
    document: &'a MapDocument,
    config: &BlockAssetConfig,
    problems: &mut Vec<String>,
) -> BTreeMap<(i32, i32), &'a MapBlockEntry> {
    let mut blocks_by_position = BTreeMap::new();
    let mut ball_count = 0;
    let mut actual_star_count = 0;
    let mut has_empty_star = false;
    let mut has_star_switch = false;

    let width = document.map_settings.size.width;
    let height = document.map_settings.size.height;

    for entry in &document.blocks {
        let position = (entry.x, entry.y);
        let block_name = entry.block.name.as_str();

        if let Some(previous) = blocks_by_position.insert(position, entry) {
            problems.push(format!(
                "blocks {} and {} occupy the same position ({}, {})",
                previous.block.name, block_name, entry.x, entry.y
            ));
        }

        if entry.x < 0 || entry.y < 0 || entry.x >= width || entry.y >= height {
            problems.push(format!(
                "block {block_name} at ({}, {}) is outside map size {}x{}",
                entry.x, entry.y, width, height
            ));
        }

        let configured_category = configured_category(config, block_name);

        match configured_category {
            Some(category) => {
                if entry.block.r#type != category.as_str() {
                    problems.push(format!(
                        "block {block_name} has category {}, expected {category}",
                        entry.block.r#type
                    ));
                }
            }
            None => {
                problems.push(format!("unknown block ID: {block_name}"));
            }
        }

        if config
            .block_exceptions
            .iter()
            .any(|block_id| block_id.as_str() == block_name)
        {
            problems.push(format!(
                "block {block_name} is an internal or non-placeable asset"
            ));
        }

        if !(0..=3).contains(&entry.block.dir) {
            problems.push(format!(
                "block {block_name} at ({}, {}) has invalid direction {}",
                entry.x, entry.y, entry.block.dir
            ));
        }

        let is_rotatable = config
            .block_rotatable
            .iter()
            .any(|block_id| block_id.as_str() == block_name);

        if entry.block.dir != 0 && !is_rotatable {
            problems.push(format!(
                "non-rotatable block {block_name} has direction {}",
                entry.block.dir
            ));
        }

        match block_name {
            BALL_ID => {
                ball_count += 1;
            }
            STAR_ID | EMPTY_STAR_ID | JUMP_STAR_ID => {
                actual_star_count += 1;

                if block_name == EMPTY_STAR_ID {
                    has_empty_star = true;
                }
            }
            STAR_SWITCH_ID => {
                has_star_switch = true;
            }
            _ => {}
        }
    }

    if ball_count != 1 {
        problems.push(format!(
            "map must contain exactly one ball, found {ball_count}"
        ));
    }

    if document.map_settings.star_count != actual_star_count {
        problems.push(format!(
            "map settings declare {} stars, but blocks contain {actual_star_count}",
            document.map_settings.star_count
        ));
    }

    if has_empty_star && !has_star_switch {
        problems.push("star_empty requires at least one wb_star_sw".to_owned());
    }

    blocks_by_position
}

fn validate_block_options(
    option_entries: Option<&[MapBlockOption]>,
    config: &BlockAssetConfig,
    blocks_by_position: &BTreeMap<(i32, i32), &MapBlockEntry>,
    problems: &mut Vec<String>,
) {
    let Some(option_entries) = option_entries else {
        return;
    };

    let mut seen_positions = BTreeSet::new();

    for option_entry in option_entries {
        let position = (option_entry.x, option_entry.y);

        if !seen_positions.insert(position) {
            problems.push(format!(
                "duplicate block options at ({}, {})",
                option_entry.x, option_entry.y
            ));
        }

        match blocks_by_position.get(&position) {
            Some(block_entry) => {
                if block_entry.block.name != option_entry.name {
                    problems.push(format!(
                        "options for {} at ({}, {}) belong to block {}",
                        option_entry.name, option_entry.x, option_entry.y, block_entry.block.name
                    ));
                }
            }
            None => {
                problems.push(format!(
                    "options for {} reference empty position ({}, {})",
                    option_entry.name, option_entry.x, option_entry.y
                ));
            }
        }

        let Some(definitions) = config.block_options.get(option_entry.name.as_str()) else {
            problems.push(format!(
                "block {} has options but no option definition",
                option_entry.name
            ));
            continue;
        };

        validate_option_values(option_entry, definitions, problems);
    }
}

fn validate_option_values(
    option_entry: &MapBlockOption,
    definitions: &[crate::block::BlockOptionDefinition],
    problems: &mut Vec<String>,
) {
    let expected_names: Vec<&str> = definitions
        .iter()
        .map(|definition| definition.value_name.as_str())
        .collect();

    let actual_names: Vec<&str> = option_entry
        .options
        .iter()
        .map(|option| option.value_name.as_str())
        .collect();

    if actual_names != expected_names {
        problems.push(format!(
            "block {} has option order {actual_names:?}, expected {expected_names:?}",
            option_entry.name
        ));
    }

    let mut seen_names = BTreeSet::new();

    for option in &option_entry.options {
        if !seen_names.insert(option.value_name.as_str()) {
            problems.push(format!(
                "block {} contains duplicate option {}",
                option_entry.name, option.value_name
            ));
        }

        let Some(definition) = definitions
            .iter()
            .find(|definition| definition.value_name == option.value_name)
        else {
            problems.push(format!(
                "block {} contains unknown option {}",
                option_entry.name, option.value_name
            ));
            continue;
        };

        if !option.value.is_finite() {
            problems.push(format!(
                "option {} on block {} is not finite",
                option.value_name, option_entry.name
            ));
        } else if option.value < definition.min || option.value > definition.max {
            problems.push(format!(
                "option {} on block {} has value {} outside {}..={}",
                option.value_name, option_entry.name, option.value, definition.min, definition.max
            ));
        }
    }
}

fn configured_category(config: &BlockAssetConfig, block_name: &str) -> Option<BlockCategory> {
    BlockCategory::ALL.into_iter().find(|category| {
        config.blocks_in(*category).is_some_and(|block_ids| {
            block_ids
                .iter()
                .any(|block_id| block_id.as_str() == block_name)
        })
    })
}
