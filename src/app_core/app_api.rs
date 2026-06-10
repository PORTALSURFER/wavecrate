//! Direct API bridge to legacy `app` modules used by migration-facing runtime layers.
//!
//! This module intentionally holds all legacy crossings in a single location while
//! `app` internals continue to be progressively extracted. The export lists below
//! are an allowlist, not a convenience facade: new app-core code should prefer
//! app-core-owned DTOs and projection helpers.
//!
//! Migration inventory:
//!
//! | Export group | Current consumers | Why it still crosses | Target owner | Removal condition | Follow-up |
//! | --- | --- | --- | --- | --- | --- |
//! | Controller runtime | `app_core::controller`, ui-projection helpers, GUI fixtures | `AppController` still owns mature browser, source, audio, waveform, map, and config behavior used by app-core runtime shims | Split Wavecrate runtime/domain services under `app_core`, with native UI depending on those contracts | Runtime dispatch, projection, and fixture construction no longer require `crate::app::controller` | `OPT-525` |
//! | Retained projection caches | `app_core::controller`, `app_core::ui_projection`, `app_core::ui_bridge` | Projection caches live with the controller state they memoize | App-core projection/cache modules with app-core-owned keys and row DTOs | Projection cache storage moves out of `AppController` | `OPT-525` |
//! | Controller dirty graph state | `app_core::controller`, `app_core::actions`, `app_core::ui_bridge` | Bridge invalidation still consumes legacy derived-node IDs and dirty reasons | App-core invalidation graph | Reducers and frame preparation own dirty-node contracts without legacy state names | `OPT-525` |
//! | Browser/source/map/audio state DTOs | `app_core::state`, `app_core::ui_projection`, `app_core::controller`, app-core tests | Projection models are still sourced from legacy `UiState` and nested browser/source/audio state structs | App-core state DTO modules and focused test builders | App-core projections/tests construct owned DTOs without importing `app_api::state` | `OPT-538` |
//! | Browser catalog/test fixtures | `app_core::actions::catalog`, ui-projection tests | Catalog samples and projection assertions still use legacy browser facet payloads | Domain-organized action catalog fixtures | Catalog fixtures are split by action domain with app-core-owned fixture builders | `OPT-539` |
//! | View-model label helper | `app_core::view_model` | Sample label formatting remains implemented in the legacy view-model helper | App-core/browser display-name helper | Display-name formatting is moved beside app-core browser DTOs | `OPT-525` |
pub(crate) mod controller {
    //! Controller runtime exports.
    //!
    //! These remain while `AppController` is the source of truth for runtime
    //! orchestration and retained projection caches. They should shrink as
    //! app-core owns runtime services and cache DTOs.

    /// Legacy application controller implementation.
    pub(crate) use crate::app::controller::AppController;
    /// Legacy retained browser preload-window cache type.
    pub(crate) use crate::app::controller::ProjectedBrowserPreloadWindow;
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
    /// Legacy GUI fixture builder used by migration-facing controller tests.
    pub(crate) use crate::app::controller::build_named_gui_fixture_controller;
    /// Legacy browser edit support predicate used by ui-projections.
    pub(crate) use crate::app::controller::supports_wav_destructive_edits;
}

pub(crate) mod controller_state {
    //! Controller-internal dirty graph and transient row-state exports.
    //!
    //! Current consumers are frame preparation, invalidation, reducers, action
    //! tests, and projection-cache tests. The replacement is an app-core-owned
    //! invalidation graph plus DTO builders for tests.

    /// Legacy active browser auto-rename row state.
    pub(crate) use crate::app::controller::state::runtime::AutoRenameBatchRowState;
    /// Legacy derived-state graph node identifiers.
    pub(crate) use crate::app::controller::state::runtime::DerivedNodeId;
    /// Legacy derived-state dirty reason categories.
    pub(crate) use crate::app::controller::state::runtime::DirtyReason;
}

pub(crate) mod state {
    //! Legacy state DTO exports used by app-core projections and tests.
    //!
    //! This allowlist documents the current migration surface. The browser,
    //! source, waveform, audio, map, prompt, progress, and drag/drop DTOs below
    //! still cross because `UiState` is projected from the legacy controller.
    //! Their intended owner is app-core state/projection modules. Remove each
    //! export when the matching app-core projection or test fixture uses an
    //! app-core-owned DTO or builder instead of `app_api::state`.

    /// Active audio picker target shown in options flows.
    pub(crate) use crate::app::state::AudioPickerTarget;
    /// Progress task DTOs still used by projection tests while OPT-538 replaces legacy state usage.
    #[cfg(test)]
    pub(crate) use crate::app::state::ProgressTaskKind;
    /// Audio option and active-device DTOs used by options-panel projection tests.
    #[cfg(test)]
    pub(crate) use crate::app::state::{ActiveAudioOutput, AudioDeviceView, AudioHostView};
    /// Browser row cleanup, search, sidebar, sort, tab, and viewport DTOs.
    pub(crate) use crate::app::state::{
        BrowserBpmFacet, BrowserDuplicateCleanupState, BrowserSidebarFilterFacet,
        BrowserSidebarFilterOption, BrowserSidebarFilterState, PlaybackAgeBucket,
        PlaybackAgeFilterChip, SampleBrowserActionPrompt, SampleBrowserSort, SampleBrowserTab,
        TagNamedFilter, TriageFlagColumn, TriageFlagFilter, VisibleRows,
        browser_playback_age_filter_chips,
    };
    /// Destructive-edit and options prompt DTOs used by prompt surfaces.
    pub(crate) use crate::app::state::{
        DestructiveEditPrompt, DestructiveSelectionEdit, OptionsPanelPrompt,
    };
    /// Drag/drop and pointer DTOs used by source, folder, waveform, and browser action dispatch.
    pub(crate) use crate::app::state::{
        DragPayload, DragSource, DragTarget, FocusContext, UiPoint,
    };
    /// Folder/source panel DTOs used by source projections and folder actions.
    pub(crate) use crate::app::state::{
        FolderActionPrompt, FolderBrowserUiState, FolderDeleteRecoveryAction,
        FolderDeleteRecoveryEntry, FolderDeleteRecoveryStatus, FolderFileScopeMode, FolderPaneId,
        FolderRowView, InlineFolderEdit, InlineFolderEditKind, RetainedFolderDeleteEntry,
    };
    /// Map projection DTOs used by map labels and map projection caches.
    pub(crate) use crate::app::state::{
        MapBounds, MapPoint, MapQueryBounds, MapRenderMode, MapSimilarityPrepStatus,
    };
    /// Browser fixture-only DTOs used by projection tests while OPT-538 replaces legacy state usage.
    #[cfg(test)]
    pub(crate) use crate::app::state::{SampleBrowserIndex, SimilarQuery};
    /// Progress and update DTOs used by app-model status projections.
    pub(crate) use crate::app::state::{StatusTone, UpdateStatus};
    /// Full UI state and waveform DTOs still projected from the legacy controller.
    pub(crate) use crate::app::state::{UiState, WaveformSliceBatchProfile};
}

pub(crate) mod view_model {
    //! Legacy view-model helpers that still back app-core display DTOs.
    //!
    //! `sample_display_label` is consumed only through `app_core::view_model`.
    //! Move it beside browser display-name DTOs once the browser projection no
    //! longer depends on the legacy app view-model module.

    /// Legacy sample display-label helper.
    pub(crate) use crate::app::view_model::sample_display_label;
}
