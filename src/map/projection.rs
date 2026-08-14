use super::{MapBlockOption, MapDocument, MapValidationErrors, validate_map_document};
use crate::{
    block::{BlockAssetConfig, BlockId},
    domain::{
        CardinalDirection, GridPosition, GridSize, InitialSwitchState, PortalPair, ValidatedBlock,
        ValidatedBlockOption, ValidatedMap, ValidatedMapSettings,
    },
};
use std::{collections::BTreeMap, error::Error, fmt};

const NULL_POSITION: (i32, i32) = (-1, -1);

#[derive(Debug)]
pub enum ValidatedMapBuildError {
    Validation(MapValidationErrors),
    ProjectionInvariant(String),
}

impl fmt::Display for ValidatedMapBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(errors) => {
                write!(formatter, "map validation failed:\n{errors}")
            }
            Self::ProjectionInvariant(message) => {
                write!(formatter, "validated map projection failed: {message}")
            }
        }
    }
}

impl Error for ValidatedMapBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Validation(errors) => Some(errors),
            Self::ProjectionInvariant(_) => None,
        }
    }
}

impl From<MapValidationErrors> for ValidatedMapBuildError {
    fn from(errors: MapValidationErrors) -> Self {
        Self::Validation(errors)
    }
}

impl ValidatedMap {
    pub fn from_document(
        document: &MapDocument,
        config: &BlockAssetConfig,
    ) -> Result<Self, ValidatedMapBuildError> {
        validate_map_document(document, config)?;

        let options_by_position: BTreeMap<GridPosition, &MapBlockOption> = document
            .block_options
            .iter()
            .flatten()
            .map(|option_entry| {
                (
                    GridPosition::new(option_entry.x, option_entry.y),
                    option_entry,
                )
            })
            .collect();

        let mut blocks = Vec::with_capacity(document.blocks.len());

        for entry in &document.blocks {
            let position = GridPosition::new(entry.x, entry.y);

            let category = config.category_for(&entry.block.name).ok_or_else(|| {
                ValidatedMapBuildError::ProjectionInvariant(format!(
                    "missing category for block {}",
                    entry.block.name
                ))
            })?;

            let direction = CardinalDirection::try_from(entry.block.dir)
                .map_err(|error| ValidatedMapBuildError::ProjectionInvariant(error.to_string()))?;

            let options =
                resolve_block_options(position, &entry.block.name, &options_by_position, config);

            blocks.push(ValidatedBlock {
                position,
                id: BlockId::from(entry.block.name.clone()),
                category,
                direction,
                options,
            });
        }

        let settings = &document.map_settings;

        let required_stars = u32::try_from(settings.star_count).map_err(|_| {
            ValidatedMapBuildError::ProjectionInvariant(format!(
                "star count {} cannot become u32",
                settings.star_count
            ))
        })?;

        Ok(Self {
            map_name: document.map_name.clone(),
            author: document.author.clone(),
            settings: ValidatedMapSettings {
                time_limit_seconds: settings.time_limit,
                show_time_ranking: settings.show_time_ranking,
                required_stars,
                size: GridSize {
                    width: settings.size.width,
                    height: settings.size.height,
                },
                teleport_1_exit: optional_position(settings.tp1_exit.x, settings.tp1_exit.y),
                teleport_2_exit: optional_position(settings.tp2_exit.x, settings.tp2_exit.y),
                portal_1: PortalPair {
                    a: optional_position(
                        settings.portal1_positions.a_px,
                        settings.portal1_positions.a_py,
                    ),
                    b: optional_position(
                        settings.portal1_positions.b_px,
                        settings.portal1_positions.b_py,
                    ),
                },
                portal_2: PortalPair {
                    a: optional_position(
                        settings.portal2_positions.a_px,
                        settings.portal2_positions.a_py,
                    ),
                    b: optional_position(
                        settings.portal2_positions.b_px,
                        settings.portal2_positions.b_py,
                    ),
                },
                initial_switches: InitialSwitchState {
                    electric: settings.sw_el,
                    block_1: settings.sw_b1,
                    block_2: settings.sw_b2,
                },
            },
            blocks,
        })
    }
}

fn optional_position(x: i32, y: i32) -> Option<GridPosition> {
    if (x, y) == NULL_POSITION {
        None
    } else {
        Some(GridPosition::new(x, y))
    }
}

fn resolve_block_options(
    position: GridPosition,
    block_name: &str,
    options_by_position: &BTreeMap<GridPosition, &MapBlockOption>,
    config: &BlockAssetConfig,
) -> Vec<ValidatedBlockOption> {
    if let Some(explicit_options) = options_by_position.get(&position) {
        return explicit_options
            .options
            .iter()
            .map(|option| ValidatedBlockOption {
                name: option.value_name.clone(),
                value: option.value,
            })
            .collect();
    }

    config
        .block_options
        .get(block_name)
        .map(|definitions| {
            definitions
                .iter()
                .map(|definition| ValidatedBlockOption {
                    name: definition.value_name.clone(),
                    value: definition.default_value,
                })
                .collect()
        })
        .unwrap_or_default()
}
