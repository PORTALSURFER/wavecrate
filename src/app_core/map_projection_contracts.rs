//! Map projection query and retained-cache contracts owned by app-core.

use crate::app_core::state::MapQueryBounds;
use crate::sample_sources::SourceId;

/// Cache key for retained map-point projection payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedMapPointsCacheKey {
    /// Stable hash of the active source identifier.
    pub source_id_hash: u64,
    /// Stable hash of the active UMAP version.
    pub umap_version_hash: u64,
    /// Monotonic revision for cached map points.
    pub points_revision: u64,
    /// Bitwise query minimum X bound.
    pub query_min_x_bits: u32,
    /// Bitwise query maximum X bound.
    pub query_max_x_bits: u32,
    /// Bitwise query minimum Y bound.
    pub query_min_y_bits: u32,
    /// Bitwise query maximum Y bound.
    pub query_max_y_bits: u32,
}

/// Retained immutable map-point payload reused across native map projections.
pub(crate) type ProjectedMapPointCacheEntry = crate::app_core::actions::NativeMapPointModel;

/// Query payload for loading visible starmap points and optional cluster metadata.
pub(crate) struct UmapPointQuery<'a> {
    pub model_id: &'a str,
    pub umap_version: &'a str,
    pub cluster_method: &'a str,
    pub cluster_umap_version: &'a str,
    pub source_id: Option<&'a SourceId>,
    pub bounds: MapQueryBounds,
    pub limit: usize,
}
