use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

/// Persisted color choices for drop target rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DropTargetColor {
    /// Mint accent color.
    Mint,
    /// Ice accent color.
    Ice,
    /// Copper accent color.
    Copper,
    /// Fog accent color.
    Fog,
    /// Amber accent color.
    Amber,
    /// Rose accent color.
    Rose,
    /// Spruce accent color.
    Spruce,
    /// Clay accent color.
    Clay,
}

/// Config data for a single drop target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropTargetConfig {
    /// Folder path that receives dropped samples.
    pub path: PathBuf,
    /// Optional display color selected for the target.
    pub color: Option<DropTargetColor>,
}

impl DropTargetConfig {
    /// Build a drop target entry for the given path, with no color assigned.
    pub fn new(path: PathBuf) -> Self {
        Self { path, color: None }
    }
}

pub(super) fn deserialize_drop_targets<'de, D>(
    deserializer: D,
) -> Result<Vec<DropTargetConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let items = Option::<Vec<DropTargetEntry>>::deserialize(deserializer)?.unwrap_or_default();
    Ok(items
        .into_iter()
        .map(DropTargetEntry::into_config)
        .collect())
}

pub(super) fn deserialize_optional_drop_targets<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<DropTargetConfig>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        Option::<Vec<DropTargetEntry>>::deserialize(deserializer)?.map(|items| {
            items
                .into_iter()
                .map(DropTargetEntry::into_config)
                .collect()
        }),
    )
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DropTargetEntry {
    Path(PathBuf),
    Config(DropTargetConfig),
}

impl DropTargetEntry {
    fn into_config(self) -> DropTargetConfig {
        match self {
            Self::Path(path) => DropTargetConfig::new(path),
            Self::Config(config) => config,
        }
    }
}
