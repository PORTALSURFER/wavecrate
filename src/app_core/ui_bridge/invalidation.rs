//! App-core projection invalidation policy.
//!
//! The UI bridge owns app-core invalidation sources/reasons and converts them
//! to the retained controller dirty graph only at the controller boundary.

mod policy;

pub(crate) use crate::app_core::invalidation_contracts::{
    InvalidationNode, InvalidationReason, InvalidationSource,
};
pub(super) use policy::{
    BROAD_DIRTY_SOURCES, action_prefers_targeted_invalidation,
    action_requires_projection_cache_invalidation, classify_dirty_source,
    waveform_render_inputs_require_refresh,
};

#[cfg(test)]
pub(crate) use policy::{catalog_dirty_source, catalog_prefers_targeted_invalidation};
