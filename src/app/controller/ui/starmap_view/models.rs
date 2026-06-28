//! Shared starmap data models used by the UI controller and repository loaders.

use crate::sample_sources::SourceId;

/// Aggregate starmap bounds for the current layout.
pub(crate) struct UmapBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

/// One rendered point from the starmap layout.
pub(crate) struct UmapPoint {
    pub sample_id: String,
    pub x: f32,
    pub y: f32,
    pub cluster_id: Option<i32>,
}

/// Query payload for loading visible starmap points and optional cluster metadata.
pub(crate) struct UmapPointQuery<'a> {
    pub model_id: &'a str,
    pub umap_version: &'a str,
    pub cluster_method: &'a str,
    pub cluster_umap_version: &'a str,
    pub source_id: Option<&'a SourceId>,
    pub bounds: crate::app::state::MapQueryBounds,
    pub limit: usize,
}
