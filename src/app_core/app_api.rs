//! Single legacy-app crossing for app-core migration adapters.
//!
//! App-core modules should import legacy controller and state contracts through
//! this file while the migration boundary still depends on the current `app`
//! implementation.
//!
//! Migration inventory:
//!
//! | Legacy surface | Active follow-up |
//! | --- | --- |
//! | Browser/source/map/audio state DTOs | `OPT-677` |

pub(crate) mod controller {
    pub(crate) use crate::app::controller::{
        AppController, ProjectedBrowserPreloadWindow, ProjectedBrowserRowCacheEntry,
        ProjectedMapPointCacheEntry, ProjectedMapPointsCacheKey, ProjectedSelectedPathsLookup,
        UmapPointQuery, build_named_gui_fixture_controller, supports_wav_destructive_edits,
    };

    pub(crate) type AutoRenameBatchRowState =
        crate::app::controller::state::runtime::AutoRenameBatchRowState;
    pub(crate) type DerivedNodeId = crate::app::controller::state::runtime::DerivedNodeId;
    pub(crate) type DirtyReason = crate::app::controller::state::runtime::DirtyReason;
}

pub(crate) mod state {
    // Test-only projection fixtures still need these legacy state DTO exports
    // while OPT-677 replaces legacy state usage behind app-core owned contracts.
    pub(crate) use crate::app::state::*;
}
