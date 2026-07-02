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
//! | App-controller runtime backend | `src/app_core::controller::runtime_facade` | App-core owned facade as of `OPT-992`; the retained legacy controller remains the backend implementation behind the facade | Done | Remaining work should add explicit facade methods instead of reintroducing `AppController` aliases through `app_api` |
//! | GUI fixture construction | `src/app_core::gui_fixtures` | App-core owned fixture adapter as of `OPT-992`; deterministic seeding still delegates to retained legacy fixture builders | Done | Remaining work should keep fixture construction out of production controller exports |
//! | WAV edit-support predicate | `src/app_core::wav_edit_support` | App-core owned projection/domain helper as of `OPT-992`; predicate implementation still delegates to retained legacy WAV library support | Done | Remaining work should move the implementation to a product-domain module when destructive-edit IO leaves the retained controller |
//! | Browser projection cache, selected-path lookup, preload-window, and auto-rename row state | `src/app_core::browser_projection_cache` | App-core owned as of `OPT-987` | Done | Remaining work should keep new browser projection contracts in app-core and avoid reintroducing bridge aliases |
//! | Map projection cache key, projected map-point entry, and UMAP point query payload | `src/app_core::map_projection_contracts` | App-core owned as of `OPT-988` | Done | Remaining work should keep new map projection contracts in app-core and avoid reintroducing bridge aliases |
//! | Dirty graph node/reason contracts used by frame preparation and invalidation adapters | `src/app_core::invalidation_contracts` | App-core owned as of `OPT-989`; conversion to the retained controller dirty graph is isolated in the contract adapter | Done | Remaining work should keep app-core invalidation names at frame-prep/bridge call sites and avoid reintroducing bridge aliases |
//! | Browser, source, folder, and library-hygiene state DTOs | `src/app_core::browser_source_state` | App-core owned as of `OPT-990`; representation aliases remain while the legacy controller stores the backing UI state | Done | Remaining work should keep browser/source/folder state imports on app-core contracts and avoid reintroducing wildcard state bridge aliases |
//! | Waveform, prompt, drag/drop, map, audio/options, progress, update, and status state DTOs | `src/app_core::projection_state` | App-core owned as of `OPT-991`; representation aliases remain while the legacy controller stores the backing UI state | Done | Remaining work should keep projection/action state imports on app-core contracts and avoid reintroducing wildcard state bridge aliases |
//!
//! No current app-api export remains after `OPT-992`; `native_app` should
//! continue consuming app-core contracts rather than owning migration DTOs
//! directly. Reintroduce aliases here only with a scoped owner and exit issue.
