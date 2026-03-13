//! Canonical GUI action catalog used by host-side tests, tools, and automation metadata.

use super::NativeUiAction;
use serde::Serialize;

/// Stable payload-free identity for one GUI action variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiActionKind {
    /// Select one top-level shell column directly.
    SelectColumn,
    /// Move the focused shell column by a relative delta.
    MoveColumn,
    /// Toggle transport playback on or off.
    ToggleTransport,
    /// Start playback from the beginning of the current sample or loop.
    PlayFromStart,
    /// Start playback from the current playhead position.
    PlayFromCurrentPlayhead,
    /// Apply the shell-wide escape-key behavior.
    HandleEscape,
    /// Move focus into the browser panel.
    FocusBrowserPanel,
    /// Move focus into the sources panel.
    FocusSourcesPanel,
    /// Move focus into the waveform panel.
    FocusWaveformPanel,
    /// Focus the currently loaded sample inside the browser list.
    FocusLoadedSampleInBrowser,
    /// Focus the browser search field.
    FocusBrowserSearch,
    /// Remove focus from the browser search field.
    BlurBrowserSearch,
    /// Open the add-source dialog.
    OpenAddSourceDialog,
    /// Open the options menu or panel.
    OpenOptionsMenu,
    /// Close the options panel.
    CloseOptionsPanel,
    /// Open the trash-folder picker flow.
    PickTrashFolder,
    /// Open the configured trash folder in the host shell.
    OpenTrashFolder,
    /// Focus the folder-search field in the sources panel.
    FocusFolderSearch,
    /// Set the sources folder-search query.
    SetFolderSearch,
    /// Select one source row directly.
    SelectSourceRow,
    /// Reload the focused source row.
    ReloadSourceRow,
    /// Force a hard sync on the focused source row.
    HardSyncSourceRow,
    /// Open the focused source folder row in the host shell.
    OpenSourceFolderRow,
    /// Remove the focused source row from the library list.
    RemoveSourceRow,
    /// Remove dead links associated with the focused source row.
    RemoveDeadLinksForSourceRow,
    /// Focus one folder row directly.
    FocusFolderRow,
    /// Move folder focus by a relative delta.
    MoveFolderFocus,
    /// Start creating a new folder under the current parent.
    StartNewFolder,
    /// Start creating a new folder at the source root.
    StartNewFolderAtRoot,
    /// Start renaming the focused folder.
    StartFolderRename,
    /// Delete the focused folder.
    DeleteFocusedFolder,
    /// Clear the folder-delete recovery log.
    ClearFolderDeleteRecoveryLog,
    /// Move browser focus by a relative row delta.
    MoveBrowserFocus,
    /// Set the top visible browser row explicitly.
    SetBrowserViewStart,
    /// Focus one browser row directly.
    FocusBrowserRow,
    /// Commit the currently focused browser row.
    CommitFocusedBrowserRow,
    /// Save the current waveform selection back to the browser/sample metadata.
    SaveWaveformSelectionToBrowser,
    /// Toggle selection on one browser row.
    ToggleBrowserRowSelection,
    /// Extend browser selection through one row.
    ExtendBrowserSelectionToRow,
    /// Add a contiguous browser-row range to the current selection.
    AddRangeBrowserSelection,
    /// Extend browser selection from the focused anchor row.
    ExtendBrowserSelectionFromFocus,
    /// Add a browser-row range from the focused anchor without clearing existing selection.
    AddRangeBrowserSelectionFromFocus,
    /// Toggle selection on the currently focused browser row.
    ToggleFocusedBrowserRowSelection,
    /// Select every visible browser row.
    SelectAllBrowserRows,
    /// Set the browser search query.
    SetBrowserSearch,
    /// Toggle one browser rating-filter chip.
    ToggleBrowserRatingFilter,
    /// Toggle random browser-navigation mode.
    ToggleRandomNavigationMode,
    /// Switch the browser between samples and map tabs.
    SetBrowserTab,
    /// Focus one sample in the map view.
    FocusMapSample,
    /// Set the active prompt input text.
    SetPromptInput,
    /// Start the browser rename flow for the focused item.
    StartBrowserRename,
    /// Confirm the active browser rename prompt.
    ConfirmBrowserRename,
    /// Cancel the active browser rename prompt.
    CancelBrowserRename,
    /// Apply a rating/tag to the current browser selection.
    TagBrowserSelection,
    /// Delete the current browser selection.
    DeleteBrowserSelection,
    /// Normalize the currently focused browser sample.
    NormalizeFocusedBrowserSample,
    /// Normalize the waveform selection or whole sample.
    NormalizeWaveformSelectionOrSample,
    /// Crop the current sample to the active waveform selection.
    CropWaveformSelection,
    /// Crop the waveform selection into a newly created sample.
    CropWaveformSelectionToNewSample,
    /// Trim away audio outside the active waveform selection.
    TrimWaveformSelection,
    /// Confirm the active prompt dialog.
    ConfirmPrompt,
    /// Cancel the active prompt dialog.
    CancelPrompt,
    /// Cancel the active progress operation.
    CancelProgress,
    /// Enable or disable live input monitoring.
    SetInputMonitoringEnabled,
    /// Enable or disable automatic advance after rating.
    SetAdvanceAfterRatingEnabled,
    /// Enable or disable destructive YOLO mode.
    SetDestructiveYoloMode,
    /// Enable or disable inverted waveform-scroll behavior.
    SetInvertWaveformScroll,
    /// Toggle loop playback for the active sample or selection.
    ToggleLoopPlayback,
    /// Switch the waveform channel-view mode.
    SetWaveformChannelView,
    /// Enable or disable normalized audition playback.
    SetNormalizedAuditionEnabled,
    /// Enable or disable BPM snap behavior.
    SetBpmSnapEnabled,
    /// Adjust BPM by a relative amount.
    AdjustWaveformBpm,
    /// Set BPM to an explicit value.
    SetWaveformBpmValue,
    /// Enable or disable transient snapping.
    SetTransientSnapEnabled,
    /// Enable or disable transient marker visibility.
    SetTransientMarkersEnabled,
    /// Enable or disable waveform slice mode.
    SetSliceModeEnabled,
    /// Set transport volume.
    SetVolume,
    /// Commit the current volume setting after an interactive edit.
    CommitVolumeSetting,
    /// Seek playback to one waveform position.
    SeekWaveform,
    /// Set the waveform cursor to one position.
    SetWaveformCursor,
    /// Begin a new waveform selection from one exact anchor point.
    BeginWaveformSelectionAt,
    /// Set the playback selection range directly.
    SetWaveformSelectionRange,
    /// Set the playback selection range while applying BPM smart-scale behavior.
    SetWaveformSelectionRangeSmartScale,
    /// Set the edit selection range directly.
    SetWaveformEditSelectionRange,
    /// Set the edit fade-in end handle.
    SetWaveformEditFadeInEnd,
    /// Set the edit fade-in mute start handle.
    SetWaveformEditFadeInMuteStart,
    /// Set the edit fade-in curve shape.
    SetWaveformEditFadeInCurve,
    /// Set the edit fade-out start handle.
    SetWaveformEditFadeOutStart,
    /// Set the edit fade-out mute end handle.
    SetWaveformEditFadeOutMuteEnd,
    /// Set the edit fade-out curve shape.
    SetWaveformEditFadeOutCurve,
    /// Finish an interactive edit-fade drag.
    FinishWaveformEditFadeDrag,
    /// Start a playback-selection drag gesture.
    StartWaveformSelectionDrag,
    /// Update an in-progress playback-selection drag gesture.
    UpdateWaveformSelectionDrag,
    /// Finish an interactive playback-selection drag.
    FinishWaveformSelectionDrag,
    /// Finish an interactive smart-scale playback-selection drag.
    FinishWaveformSelectionSmartScaleDrag,
    /// Begin shifting the playback selection without resizing it.
    BeginWaveformSelectionShift,
    /// Begin shifting the edit selection without resizing it.
    BeginWaveformEditSelectionShift,
    /// Clear the active playback selection.
    ClearWaveformSelection,
    /// Clear the active edit selection.
    ClearWaveformEditSelection,
    /// Clear both playback and edit selections together.
    ClearWaveformSelections,
    /// Center the waveform viewport on one position.
    SetWaveformViewCenter,
    /// Zoom the waveform viewport by a relative amount.
    ZoomWaveform,
    /// Zoom the waveform viewport to the active selection.
    ZoomWaveformToSelection,
    /// Reset the waveform viewport to the full sample.
    ZoomWaveformFull,
    /// Undo the last reversible user action.
    Undo,
    /// Redo the last undone user action.
    Redo,
    /// Start the check-for-updates flow.
    CheckForUpdates,
    /// Open the selected update or release link.
    OpenUpdateLink,
    /// Start installing the selected update.
    InstallUpdate,
    /// Dismiss the active update prompt or panel.
    DismissUpdate,
}

impl GuiActionKind {
    /// All currently cataloged action kinds in stable declaration order.
    pub const ALL: [Self; 110] = [
        Self::SelectColumn,
        Self::MoveColumn,
        Self::ToggleTransport,
        Self::PlayFromStart,
        Self::PlayFromCurrentPlayhead,
        Self::HandleEscape,
        Self::FocusBrowserPanel,
        Self::FocusSourcesPanel,
        Self::FocusWaveformPanel,
        Self::FocusLoadedSampleInBrowser,
        Self::FocusBrowserSearch,
        Self::BlurBrowserSearch,
        Self::OpenAddSourceDialog,
        Self::OpenOptionsMenu,
        Self::CloseOptionsPanel,
        Self::PickTrashFolder,
        Self::OpenTrashFolder,
        Self::FocusFolderSearch,
        Self::SetFolderSearch,
        Self::SelectSourceRow,
        Self::ReloadSourceRow,
        Self::HardSyncSourceRow,
        Self::OpenSourceFolderRow,
        Self::RemoveSourceRow,
        Self::RemoveDeadLinksForSourceRow,
        Self::FocusFolderRow,
        Self::MoveFolderFocus,
        Self::StartNewFolder,
        Self::StartNewFolderAtRoot,
        Self::StartFolderRename,
        Self::DeleteFocusedFolder,
        Self::ClearFolderDeleteRecoveryLog,
        Self::MoveBrowserFocus,
        Self::SetBrowserViewStart,
        Self::FocusBrowserRow,
        Self::CommitFocusedBrowserRow,
        Self::SaveWaveformSelectionToBrowser,
        Self::ToggleBrowserRowSelection,
        Self::ExtendBrowserSelectionToRow,
        Self::AddRangeBrowserSelection,
        Self::ExtendBrowserSelectionFromFocus,
        Self::AddRangeBrowserSelectionFromFocus,
        Self::ToggleFocusedBrowserRowSelection,
        Self::SelectAllBrowserRows,
        Self::SetBrowserSearch,
        Self::ToggleBrowserRatingFilter,
        Self::ToggleRandomNavigationMode,
        Self::SetBrowserTab,
        Self::FocusMapSample,
        Self::SetPromptInput,
        Self::StartBrowserRename,
        Self::ConfirmBrowserRename,
        Self::CancelBrowserRename,
        Self::TagBrowserSelection,
        Self::DeleteBrowserSelection,
        Self::NormalizeFocusedBrowserSample,
        Self::NormalizeWaveformSelectionOrSample,
        Self::CropWaveformSelection,
        Self::CropWaveformSelectionToNewSample,
        Self::TrimWaveformSelection,
        Self::ConfirmPrompt,
        Self::CancelPrompt,
        Self::CancelProgress,
        Self::SetInputMonitoringEnabled,
        Self::SetAdvanceAfterRatingEnabled,
        Self::SetDestructiveYoloMode,
        Self::SetInvertWaveformScroll,
        Self::ToggleLoopPlayback,
        Self::SetWaveformChannelView,
        Self::SetNormalizedAuditionEnabled,
        Self::SetBpmSnapEnabled,
        Self::AdjustWaveformBpm,
        Self::SetWaveformBpmValue,
        Self::SetTransientSnapEnabled,
        Self::SetTransientMarkersEnabled,
        Self::SetSliceModeEnabled,
        Self::SetVolume,
        Self::CommitVolumeSetting,
        Self::SeekWaveform,
        Self::SetWaveformCursor,
        Self::BeginWaveformSelectionAt,
        Self::SetWaveformSelectionRange,
        Self::SetWaveformSelectionRangeSmartScale,
        Self::SetWaveformEditSelectionRange,
        Self::SetWaveformEditFadeInEnd,
        Self::SetWaveformEditFadeInMuteStart,
        Self::SetWaveformEditFadeInCurve,
        Self::SetWaveformEditFadeOutStart,
        Self::SetWaveformEditFadeOutMuteEnd,
        Self::SetWaveformEditFadeOutCurve,
        Self::FinishWaveformEditFadeDrag,
        Self::StartWaveformSelectionDrag,
        Self::UpdateWaveformSelectionDrag,
        Self::FinishWaveformSelectionDrag,
        Self::FinishWaveformSelectionSmartScaleDrag,
        Self::BeginWaveformSelectionShift,
        Self::BeginWaveformEditSelectionShift,
        Self::ClearWaveformSelection,
        Self::ClearWaveformEditSelection,
        Self::ClearWaveformSelections,
        Self::SetWaveformViewCenter,
        Self::ZoomWaveform,
        Self::ZoomWaveformToSelection,
        Self::ZoomWaveformFull,
        Self::Undo,
        Self::Redo,
        Self::CheckForUpdates,
        Self::OpenUpdateLink,
        Self::InstallUpdate,
        Self::DismissUpdate,
    ];
}

/// GUI ownership surface used for coverage and automation planning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiSurface {
    /// Browser list, tabs, filters, and related sample browsing controls.
    Browser,
    /// Source and folder management controls.
    Sources,
    /// Waveform view, transport-adjacent edits, and zoom/selection controls.
    Waveform,
    /// Global playback transport and volume controls.
    Transport,
    /// Two-dimensional sample map surface.
    Map,
    /// Options and settings panel surface.
    Options,
    /// Prompt or confirmation dialog surface.
    Prompt,
    /// Update notification and installer surface.
    Update,
}

/// Expected effect class used to group contract expectations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiEffectClass {
    /// Action mutates state without expected background IO or visible motion.
    StateOnly,
    /// Action rebuilds or materially changes the projected UI model.
    Projection,
    /// Action is primarily a high-frequency runtime-motion interaction.
    RuntimeMotion,
    /// Action starts or interacts with an IO-backed job or background task.
    IoJob,
    /// Action can delete or irreversibly modify user data.
    Destructive,
}

/// Required coverage layer for one GUI action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiCoverageLayer {
    /// Semantic automation snapshot coverage for stable node/action contracts.
    SemanticContract,
    /// Native runtime input-routing coverage.
    RuntimeInput,
    /// App-core or bridge projection snapshot coverage.
    ProjectionSnapshot,
    /// Desktop AIV coverage against the live Windows application.
    DesktopAiv,
}

/// Host-owned coverage metadata for one `UiAction` variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct GuiActionCatalogEntry {
    /// Stable payload-free action identity.
    pub kind: GuiActionKind,
    /// Stable action identifier used in reports and automation metadata.
    pub action_id: &'static str,
    /// Top-level GUI ownership surface for the action.
    pub surface: GuiSurface,
    /// Expected effect class for the action.
    pub effect_class: GuiEffectClass,
    /// Coverage layers that must exist for the action.
    pub coverage_layers: &'static [GuiCoverageLayer],
    /// Default fixture or scenario tags to seed targeted suites.
    pub default_fixture_tags: &'static [&'static str],
}

macro_rules! gui_action_catalog {
    ($(
        $kind:ident $pattern:tt => {
            id: $id:literal,
            surface: $surface:ident,
            effect: $effect:ident,
            coverage: [$($coverage:ident),+ $(,)?],
            fixtures: [$($fixture:literal),* $(,)?],
            sample: $sample:expr
        }
    ),+ $(,)?) => {
        /// Canonical host-side GUI action catalog.
        pub const GUI_ACTION_CATALOG: &[GuiActionCatalogEntry] = &[
            $(
                GuiActionCatalogEntry {
                    kind: GuiActionKind::$kind,
                    action_id: $id,
                    surface: GuiSurface::$surface,
                    effect_class: GuiEffectClass::$effect,
                    coverage_layers: &[$(GuiCoverageLayer::$coverage),+],
                    default_fixture_tags: &[$($fixture),*],
                },
            )+
        ];

        /// Return the payload-free kind for one concrete native UI action.
        pub fn action_kind(action: &NativeUiAction) -> GuiActionKind {
            match action {
                $(gui_action_catalog!(@match $kind $pattern) => GuiActionKind::$kind,)+
            }
        }

        /// Return a representative action payload for the provided kind.
        pub fn representative_action_for_kind(kind: GuiActionKind) -> NativeUiAction {
            match kind {
                $(GuiActionKind::$kind => $sample,)+
            }
        }
    };
    (@match $kind:ident {}) => {
        NativeUiAction::$kind
    };
    (@match $kind:ident { $($field:ident),+ }) => {
        NativeUiAction::$kind { $($field: _),+ }
    };
}

gui_action_catalog!(
    SelectColumn { index } => { id: "select_column", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::SelectColumn { index: 1 } },
    MoveColumn { delta } => { id: "move_column", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::MoveColumn { delta: 1 } },
    ToggleTransport {} => { id: "toggle_transport", surface: Transport, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["transport"], sample: NativeUiAction::ToggleTransport },
    PlayFromStart {} => { id: "play_from_start", surface: Transport, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["transport"], sample: NativeUiAction::PlayFromStart },
    PlayFromCurrentPlayhead {} => { id: "play_from_current_playhead", surface: Transport, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["transport"], sample: NativeUiAction::PlayFromCurrentPlayhead },
    HandleEscape {} => { id: "handle_escape", surface: Prompt, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["prompt"], sample: NativeUiAction::HandleEscape },
    FocusBrowserPanel {} => { id: "focus_browser_panel", surface: Browser, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["browser"], sample: NativeUiAction::FocusBrowserPanel },
    FocusSourcesPanel {} => { id: "focus_sources_panel", surface: Sources, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::FocusSourcesPanel },
    FocusWaveformPanel {} => { id: "focus_waveform_panel", surface: Waveform, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["waveform"], sample: NativeUiAction::FocusWaveformPanel },
    FocusLoadedSampleInBrowser {} => { id: "focus_loaded_sample_in_browser", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusLoadedSampleInBrowser },
    FocusBrowserSearch {} => { id: "focus_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser", "search"], sample: NativeUiAction::FocusBrowserSearch },
    BlurBrowserSearch {} => { id: "blur_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "search"], sample: NativeUiAction::BlurBrowserSearch },
    OpenAddSourceDialog {} => { id: "open_add_source_dialog", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::OpenAddSourceDialog },
    OpenOptionsMenu {} => { id: "open_options_menu", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["options"], sample: NativeUiAction::OpenOptionsMenu },
    CloseOptionsPanel {} => { id: "close_options_panel", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["options"], sample: NativeUiAction::CloseOptionsPanel },
    PickTrashFolder {} => { id: "pick_trash_folder", surface: Options, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::PickTrashFolder },
    OpenTrashFolder {} => { id: "open_trash_folder", surface: Options, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenTrashFolder },
    FocusFolderSearch {} => { id: "focus_folder_search", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusFolderSearch },
    SetFolderSearch { query } => { id: "set_folder_search", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::SetFolderSearch { query: String::from("drums") } },
    SelectSourceRow { index } => { id: "select_source_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["sources"], sample: NativeUiAction::SelectSourceRow { index: 0 } },
    ReloadSourceRow { index } => { id: "reload_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ReloadSourceRow { index: 0 } },
    HardSyncSourceRow { index } => { id: "hard_sync_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::HardSyncSourceRow { index: 0 } },
    OpenSourceFolderRow { index } => { id: "open_source_folder_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::OpenSourceFolderRow { index: 0 } },
    RemoveSourceRow { index } => { id: "remove_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveSourceRow { index: 0 } },
    RemoveDeadLinksForSourceRow { index } => { id: "remove_dead_links_for_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveDeadLinksForSourceRow { index: 0 } },
    FocusFolderRow { index } => { id: "focus_folder_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusFolderRow { index: 0 } },
    MoveFolderFocus { delta } => { id: "move_folder_focus", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::MoveFolderFocus { delta: 1 } },
    StartNewFolder {} => { id: "start_new_folder", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartNewFolder },
    StartNewFolderAtRoot {} => { id: "start_new_folder_at_root", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartNewFolderAtRoot },
    StartFolderRename {} => { id: "start_folder_rename", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartFolderRename },
    DeleteFocusedFolder {} => { id: "delete_focused_folder", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::DeleteFocusedFolder },
    ClearFolderDeleteRecoveryLog {} => { id: "clear_folder_delete_recovery_log", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ClearFolderDeleteRecoveryLog },
    MoveBrowserFocus { delta } => { id: "move_browser_focus", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser"], sample: NativeUiAction::MoveBrowserFocus { delta: 1 } },
    SetBrowserViewStart { visible_row } => { id: "set_browser_view_start", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::SetBrowserViewStart { visible_row: 4 } },
    FocusBrowserRow { visible_row } => { id: "focus_browser_row", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser"], sample: NativeUiAction::FocusBrowserRow { visible_row: 2 } },
    CommitFocusedBrowserRow {} => { id: "commit_focused_browser_row", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser"], sample: NativeUiAction::CommitFocusedBrowserRow },
    SaveWaveformSelectionToBrowser {} => { id: "save_waveform_selection_to_browser", surface: Waveform, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform", "browser"], sample: NativeUiAction::SaveWaveformSelectionToBrowser },
    ToggleBrowserRowSelection { visible_row } => { id: "toggle_browser_row_selection", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserRowSelection { visible_row: 2 } },
    ExtendBrowserSelectionToRow { visible_row } => { id: "extend_browser_selection_to_row", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ExtendBrowserSelectionToRow { visible_row: 3 } },
    AddRangeBrowserSelection { visible_row } => { id: "add_range_browser_selection", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::AddRangeBrowserSelection { visible_row: 3 } },
    ExtendBrowserSelectionFromFocus { delta } => { id: "extend_browser_selection_from_focus", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ExtendBrowserSelectionFromFocus { delta: 1 } },
    AddRangeBrowserSelectionFromFocus { delta } => { id: "add_range_browser_selection_from_focus", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::AddRangeBrowserSelectionFromFocus { delta: 1 } },
    ToggleFocusedBrowserRowSelection {} => { id: "toggle_focused_browser_row_selection", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleFocusedBrowserRowSelection },
    SelectAllBrowserRows {} => { id: "select_all_browser_rows", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::SelectAllBrowserRows },
    SetBrowserSearch { query } => { id: "set_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser", "search"], sample: NativeUiAction::SetBrowserSearch { query: String::from("kick") } },
    ToggleBrowserRatingFilter { level, invert } => { id: "toggle_browser_rating_filter", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserRatingFilter { level: 3, invert: false } },
    ToggleRandomNavigationMode {} => { id: "toggle_random_navigation_mode", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleRandomNavigationMode },
    SetBrowserTab { map } => { id: "set_browser_tab", surface: Map, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser", "map"], sample: NativeUiAction::SetBrowserTab { map: true } },
    FocusMapSample { sample_id } => { id: "focus_map_sample", surface: Map, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["map"], sample: NativeUiAction::FocusMapSample { sample_id: String::from("sample-1") } },
    SetPromptInput { value } => { id: "set_prompt_input", surface: Prompt, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["prompt"], sample: NativeUiAction::SetPromptInput { value: String::from("renamed") } },
    StartBrowserRename {} => { id: "start_browser_rename", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "prompt"], sample: NativeUiAction::StartBrowserRename },
    ConfirmBrowserRename {} => { id: "confirm_browser_rename", surface: Browser, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "prompt"], sample: NativeUiAction::ConfirmBrowserRename },
    CancelBrowserRename {} => { id: "cancel_browser_rename", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "prompt"], sample: NativeUiAction::CancelBrowserRename },
    TagBrowserSelection { target } => { id: "tag_browser_selection", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::TagBrowserSelection { target: radiant::app::BrowserTagTarget::Keep } },
    DeleteBrowserSelection {} => { id: "delete_browser_selection", surface: Browser, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::DeleteBrowserSelection },
    NormalizeFocusedBrowserSample {} => { id: "normalize_focused_browser_sample", surface: Browser, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::NormalizeFocusedBrowserSample },
    NormalizeWaveformSelectionOrSample {} => { id: "normalize_waveform_selection_or_sample", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::NormalizeWaveformSelectionOrSample },
    CropWaveformSelection {} => { id: "crop_waveform_selection", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::CropWaveformSelection },
    CropWaveformSelectionToNewSample {} => { id: "crop_waveform_selection_to_new_sample", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::CropWaveformSelectionToNewSample },
    TrimWaveformSelection {} => { id: "trim_waveform_selection", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::TrimWaveformSelection },
    ConfirmPrompt {} => { id: "confirm_prompt", surface: Prompt, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["prompt"], sample: NativeUiAction::ConfirmPrompt },
    CancelPrompt {} => { id: "cancel_prompt", surface: Prompt, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["prompt"], sample: NativeUiAction::CancelPrompt },
    CancelProgress {} => { id: "cancel_progress", surface: Prompt, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["progress"], sample: NativeUiAction::CancelProgress },
    SetInputMonitoringEnabled { enabled } => { id: "set_input_monitoring_enabled", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetInputMonitoringEnabled { enabled: true } },
    SetAdvanceAfterRatingEnabled { enabled } => { id: "set_advance_after_rating_enabled", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAdvanceAfterRatingEnabled { enabled: true } },
    SetDestructiveYoloMode { enabled } => { id: "set_destructive_yolo_mode", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetDestructiveYoloMode { enabled: true } },
    SetInvertWaveformScroll { enabled } => { id: "set_invert_waveform_scroll", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetInvertWaveformScroll { enabled: true } },
    ToggleLoopPlayback {} => { id: "toggle_loop_playback", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::ToggleLoopPlayback },
    SetWaveformChannelView { stereo } => { id: "set_waveform_channel_view", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformChannelView { stereo: true } },
    SetNormalizedAuditionEnabled { enabled } => { id: "set_normalized_audition_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetNormalizedAuditionEnabled { enabled: true } },
    SetBpmSnapEnabled { enabled } => { id: "set_bpm_snap_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetBpmSnapEnabled { enabled: true } },
    AdjustWaveformBpm { delta } => { id: "adjust_waveform_bpm", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::AdjustWaveformBpm { delta: 1 } },
    SetWaveformBpmValue { value_tenths } => { id: "set_waveform_bpm_value", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformBpmValue { value_tenths: 1280 } },
    SetTransientSnapEnabled { enabled } => { id: "set_transient_snap_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetTransientSnapEnabled { enabled: true } },
    SetTransientMarkersEnabled { enabled } => { id: "set_transient_markers_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetTransientMarkersEnabled { enabled: true } },
    SetSliceModeEnabled { enabled } => { id: "set_slice_mode_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetSliceModeEnabled { enabled: true } },
    SetVolume { value_milli } => { id: "set_volume", surface: Transport, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["transport"], sample: NativeUiAction::SetVolume { value_milli: 750 } },
    CommitVolumeSetting {} => { id: "commit_volume_setting", surface: Transport, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["transport"], sample: NativeUiAction::CommitVolumeSetting },
    SeekWaveform { position_milli } => { id: "seek_waveform", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::SeekWaveform { position_milli: 450 } },
    SetWaveformCursor { position_milli } => { id: "set_waveform_cursor", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformCursor { position_milli: 450 } },
    BeginWaveformSelectionAt { anchor_micros } => { id: "begin_waveform_selection_at", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::BeginWaveformSelectionAt { anchor_micros: 125_000 } },
    SetWaveformSelectionRange { start_micros, end_micros, preserve_view_edge } => { id: "set_waveform_selection_range", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformSelectionRange { start_micros: 100_000, end_micros: 300_000, preserve_view_edge: false } },
    SetWaveformSelectionRangeSmartScale { start_micros, end_micros } => { id: "set_waveform_selection_range_smart_scale", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformSelectionRangeSmartScale { start_micros: 100_000, end_micros: 300_000 } },
    SetWaveformEditSelectionRange { start_micros, end_micros, preserve_view_edge } => { id: "set_waveform_edit_selection_range", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformEditSelectionRange { start_micros: 100_000, end_micros: 300_000, preserve_view_edge: false } },
    SetWaveformEditFadeInEnd { position_micros } => { id: "set_waveform_edit_fade_in_end", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformEditFadeInEnd { position_micros: 120_000 } },
    SetWaveformEditFadeInMuteStart { position_micros } => { id: "set_waveform_edit_fade_in_mute_start", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformEditFadeInMuteStart { position_micros: 110_000 } },
    SetWaveformEditFadeInCurve { curve_milli } => { id: "set_waveform_edit_fade_in_curve", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformEditFadeInCurve { curve_milli: 500 } },
    SetWaveformEditFadeOutStart { position_micros } => { id: "set_waveform_edit_fade_out_start", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformEditFadeOutStart { position_micros: 700_000 } },
    SetWaveformEditFadeOutMuteEnd { position_micros } => { id: "set_waveform_edit_fade_out_mute_end", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformEditFadeOutMuteEnd { position_micros: 720_000 } },
    SetWaveformEditFadeOutCurve { curve_milli } => { id: "set_waveform_edit_fade_out_curve", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformEditFadeOutCurve { curve_milli: 500 } },
    FinishWaveformEditFadeDrag {} => { id: "finish_waveform_edit_fade_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformEditFadeDrag },
    StartWaveformSelectionDrag { pointer_x, pointer_y } => { id: "start_waveform_selection_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::StartWaveformSelectionDrag { pointer_x: 120, pointer_y: 80 } },
    UpdateWaveformSelectionDrag { pointer_x, pointer_y, over_browser_list, shift_down, alt_down } => { id: "update_waveform_selection_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::UpdateWaveformSelectionDrag { pointer_x: 180, pointer_y: 80, over_browser_list: false, shift_down: false, alt_down: false } },
    FinishWaveformSelectionDrag {} => { id: "finish_waveform_selection_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformSelectionDrag },
    FinishWaveformSelectionSmartScaleDrag {} => { id: "finish_waveform_selection_smart_scale_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformSelectionSmartScaleDrag },
    BeginWaveformSelectionShift { pointer_micros, start_micros, end_micros } => { id: "begin_waveform_selection_shift", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::BeginWaveformSelectionShift { pointer_micros: 200_000, start_micros: 100_000, end_micros: 300_000 } },
    BeginWaveformEditSelectionShift { pointer_micros, start_micros, end_micros } => { id: "begin_waveform_edit_selection_shift", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::BeginWaveformEditSelectionShift { pointer_micros: 200_000, start_micros: 100_000, end_micros: 300_000 } },
    ClearWaveformSelection {} => { id: "clear_waveform_selection", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ClearWaveformSelection },
    ClearWaveformEditSelection {} => { id: "clear_waveform_edit_selection", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ClearWaveformEditSelection },
    ClearWaveformSelections {} => { id: "clear_waveform_selections", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::ClearWaveformSelections },
    SetWaveformViewCenter { center_micros } => { id: "set_waveform_view_center", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformViewCenter { center_micros: 500_000 } },
    ZoomWaveform { zoom_in, steps, anchor_ratio_micros } => { id: "zoom_waveform", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::ZoomWaveform { zoom_in: true, steps: 1, anchor_ratio_micros: Some(500_000) } },
    ZoomWaveformToSelection {} => { id: "zoom_waveform_to_selection", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ZoomWaveformToSelection },
    ZoomWaveformFull {} => { id: "zoom_waveform_full", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ZoomWaveformFull },
    Undo {} => { id: "undo", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::Undo },
    Redo {} => { id: "redo", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::Redo },
    CheckForUpdates {} => { id: "check_for_updates", surface: Update, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["update"], sample: NativeUiAction::CheckForUpdates },
    OpenUpdateLink {} => { id: "open_update_link", surface: Update, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["update"], sample: NativeUiAction::OpenUpdateLink },
    InstallUpdate {} => { id: "install_update", surface: Update, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["update"], sample: NativeUiAction::InstallUpdate },
    DismissUpdate {} => { id: "dismiss_update", surface: Update, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["update"], sample: NativeUiAction::DismissUpdate }
);

/// Return the catalog entry for one concrete native action.
pub fn action_catalog_entry(action: &NativeUiAction) -> &'static GuiActionCatalogEntry {
    action_catalog_entry_by_kind(action_kind(action))
}

/// Resolve one catalog entry by stable action identifier.
pub fn action_catalog_entry_by_id(action_id: &str) -> Option<&'static GuiActionCatalogEntry> {
    GUI_ACTION_CATALOG
        .iter()
        .find(|entry| entry.action_id == action_id)
}

/// Resolve one catalog entry by action kind.
pub fn action_catalog_entry_by_kind(kind: GuiActionKind) -> &'static GuiActionCatalogEntry {
    GUI_ACTION_CATALOG
        .iter()
        .find(|entry| entry.kind == kind)
        .unwrap_or_else(|| panic!("missing GUI action catalog entry for {kind:?}"))
}
