//! Single legacy-app crossing for app-core migration adapters.
//!
//! App-core modules should import legacy controller and state contracts through
//! this file while the migration boundary still depends on the current `app`
//! implementation.
//!
//! Current migration inventory:
//!
//! | Legacy surface | Current owner | Classification | Exit issue | Exit criteria |
//! | --- | --- | --- | --- | --- |
//! | `AppController`, startup/test fixture helpers, and WAV edit-support predicate | `src/app::controller` | Intentionally still owned by `src/app` while mature runtime behavior remains there | `OPT-992` | App-core exposes a narrow runtime facade; fixture construction is test-owned; domain helpers no longer require broad controller aliases |
//! | Browser projection cache, selected-path lookup, preload-window, and auto-rename row state | `src/app::controller` retained projection state | Ready for app-core ownership | `OPT-987` | Browser projection helpers consume app-core-owned cache/row contracts and the matching aliases leave this bridge |
//! | Map projection cache key, projected map-point entry, and UMAP point query payload | `src/app::controller` retained projection/query state | Ready for app-core ownership | `OPT-988` | Map projection consumes app-core-owned query/cache contracts or a narrow map runtime adapter |
//! | Dirty graph node/reason aliases used by frame preparation and invalidation adapters | `src/app::controller::state::runtime` | Ready for app-core invalidation ownership | `OPT-989` | App-core frame preparation and bridge invalidation use app-core invalidation names; legacy conversion is isolated or removed |
//! | Browser, source, folder, and library-hygiene state DTOs exposed through the wildcard state bridge | `src/app::state` | Ready for app-core state ownership | `OPT-990` | Browser/source/folder projections and tests use app-core DTOs or focused builders |
//! | Waveform, prompt, drag/drop, map, audio/options, progress, update, and status state DTOs exposed through the wildcard state bridge | `src/app::state` | Ready for app-core state ownership | `OPT-991` | Covered projection/action consumers use app-core DTOs or focused builders |
//!
//! No current app-api export was classified as ready for direct `native_app`
//! ownership in the `OPT-949` audit; `native_app` should continue consuming
//! app-core contracts rather than owning these migration DTOs directly. No
//! alias was confirmed obsolete/dead in this audit. Remove aliases only after a
//! scoped follow-up proves the replacement path with focused validation.

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
    // Compatibility exports for state DTOs that are being moved in scoped
    // slices: browser/source/folder through OPT-990 and the remaining
    // waveform/prompt/drag/map/audio/status groups through OPT-991.
    pub(crate) use crate::app::state::*;
}
