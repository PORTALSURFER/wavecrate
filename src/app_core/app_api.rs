//! Direct API bridge to legacy `app` modules used by migration-facing runtime layers.
//!
//! This module intentionally holds all legacy crossings in a single location while
//! `app` internals continue to be progressively extracted.
pub(crate) mod controller {
    /// Legacy application controller implementation.
    pub(crate) use crate::app::controller::AppController;
    /// Legacy retained browser-row projection cache entry type.
    pub(crate) use crate::app::controller::ProjectedBrowserRowCacheEntry;
    /// Legacy retained normalized map-point projection cache entry type.
    pub(crate) use crate::app::controller::ProjectedMapPointCacheEntry;
    /// Legacy retained map-point projection cache key type.
    pub(crate) use crate::app::controller::ProjectedMapPointsCacheKey;
    /// Legacy retained selected-path lookup projection cache type.
    pub(crate) use crate::app::controller::ProjectedSelectedPathsLookup;
    /// Legacy map-point query payload used by map projection loading.
    pub(crate) use crate::app::controller::UmapPointQuery;
}

/// Legacy controller-internal state types needed by migration glue.
pub(crate) mod controller_state {
    /// Legacy derived-state graph node identifiers.
    pub(crate) use crate::app::controller::state::runtime::DerivedNodeId;
    /// Legacy derived-state dirty reason categories.
    pub(crate) use crate::app::controller::state::runtime::DirtyReason;
}

pub(crate) mod state {
    /// Legacy application state types.
    pub(crate) use crate::app::state::*;
}

pub(crate) mod view_model {
    /// Legacy sample view-model helpers.
    pub(crate) use crate::app::view_model::*;
}
