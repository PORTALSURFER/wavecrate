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
//! | Browser projection cache, selected-path lookup, preload-window, and auto-rename row state | `src/app_core::browser_projection_cache` | App-core owned as of `OPT-987` | Done | Remaining work should keep new browser projection contracts in app-core and avoid reintroducing bridge aliases |
//! | Map projection cache key, projected map-point entry, and UMAP point query payload | `src/app_core::map_projection_contracts` | App-core owned as of `OPT-988` | Done | Remaining work should keep new map projection contracts in app-core and avoid reintroducing bridge aliases |
//! | Dirty graph node/reason contracts used by frame preparation and invalidation adapters | `src/app_core::invalidation_contracts` | App-core owned as of `OPT-989`; conversion to the retained controller dirty graph is isolated in the contract adapter | Done | Remaining work should keep app-core invalidation names at frame-prep/bridge call sites and avoid reintroducing bridge aliases |
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
        AppController, build_named_gui_fixture_controller, supports_wav_destructive_edits,
    };
}

pub(crate) mod state {
    // Compatibility exports for state DTOs that are being moved in scoped
    // slices: browser/source/folder through OPT-990 and the remaining
    // waveform/prompt/drag/map/audio/status groups through OPT-991.
    pub(crate) use crate::app::state::*;
}
