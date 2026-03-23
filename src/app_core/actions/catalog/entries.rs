//! Stable GUI action catalog entries and lookup helpers.

use super::super::NativeUiAction;
use super::{GuiActionCatalogEntry, GuiActionKind, GuiCoverageLayer, GuiEffectClass, GuiSurface};

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
    FocusFolderPanel {} => { id: "focus_folder_panel", surface: Sources, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::FocusFolderPanel },
    FocusLoadedSampleInBrowser {} => { id: "focus_loaded_sample_in_browser", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusLoadedSampleInBrowser },
    FocusBrowserSearch {} => { id: "focus_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "search"], sample: NativeUiAction::FocusBrowserSearch },
    BlurBrowserSearch {} => { id: "blur_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "search"], sample: NativeUiAction::BlurBrowserSearch },
    OpenAddSourceDialog {} => { id: "open_add_source_dialog", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::OpenAddSourceDialog },
    OpenOptionsMenu {} => { id: "open_options_menu", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenOptionsMenu },
    CloseOptionsPanel {} => { id: "close_options_panel", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["options"], sample: NativeUiAction::CloseOptionsPanel },
    PickTrashFolder {} => { id: "pick_trash_folder", surface: Options, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::PickTrashFolder },
    OpenTrashFolder {} => { id: "open_trash_folder", surface: Options, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenTrashFolder },
    FocusFolderSearch {} => { id: "focus_folder_search", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusFolderSearch },
    SetFolderSearch { query } => { id: "set_folder_search", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::SetFolderSearch { query: String::from("drums") } },
    FocusSourceRow { index } => { id: "focus_source_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusSourceRow { index: 0 } },
    SelectSourceRow { index } => { id: "select_source_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::SelectSourceRow { index: 0 } },
    MoveSourceFocus { delta } => { id: "move_source_focus", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::MoveSourceFocus { delta: 1 } },
    ReloadFocusedSourceRow {} => { id: "reload_focused_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ReloadFocusedSourceRow },
    HardSyncFocusedSourceRow {} => { id: "hard_sync_focused_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::HardSyncFocusedSourceRow },
    OpenFocusedSourceFolder {} => { id: "open_focused_source_folder", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::OpenFocusedSourceFolder },
    RemoveFocusedSourceRow {} => { id: "remove_focused_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveFocusedSourceRow },
    RemoveDeadLinksForFocusedSourceRow {} => { id: "remove_dead_links_for_focused_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveDeadLinksForFocusedSourceRow },
    ReloadSourceRow { index } => { id: "reload_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ReloadSourceRow { index: 0 } },
    HardSyncSourceRow { index } => { id: "hard_sync_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::HardSyncSourceRow { index: 0 } },
    OpenSourceFolderRow { index } => { id: "open_source_folder_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::OpenSourceFolderRow { index: 0 } },
    RemoveSourceRow { index } => { id: "remove_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveSourceRow { index: 0 } },
    RemoveDeadLinksForSourceRow { index } => { id: "remove_dead_links_for_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveDeadLinksForSourceRow { index: 0 } },
    FocusFolderRow { index } => { id: "focus_folder_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusFolderRow { index: 0 } },
    ToggleFocusedFolderSelection {} => { id: "toggle_focused_folder_selection", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ToggleFocusedFolderSelection },
    MoveFolderFocus { delta } => { id: "move_folder_focus", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::MoveFolderFocus { delta: 1 } },
    StartNewFolder {} => { id: "start_new_folder", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartNewFolder },
    StartNewFolderAtRoot {} => { id: "start_new_folder_at_root", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartNewFolderAtRoot },
    StartFolderRename {} => { id: "start_folder_rename", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartFolderRename },
    DeleteFocusedFolder {} => { id: "delete_focused_folder", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::DeleteFocusedFolder },
    ClearFolderDeleteRecoveryLog {} => { id: "clear_folder_delete_recovery_log", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ClearFolderDeleteRecoveryLog },
    MoveBrowserFocus { delta } => { id: "move_browser_focus", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::MoveBrowserFocus { delta: 1 } },
    SetBrowserViewStart { visible_row } => { id: "set_browser_view_start", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::SetBrowserViewStart { visible_row: 4 } },
    FocusBrowserRow { visible_row } => { id: "focus_browser_row", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusBrowserRow { visible_row: 2 } },
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
    FocusPreviousBrowserHistory {} => { id: "focus_previous_browser_history", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusPreviousBrowserHistory },
    FocusNextBrowserHistory {} => { id: "focus_next_browser_history", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusNextBrowserHistory },
    ToggleFindSimilarFocusedSample {} => { id: "toggle_find_similar_focused_sample", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleFindSimilarFocusedSample },
    PlayRandomSample {} => { id: "play_random_sample", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::PlayRandomSample },
    PlayPreviousRandomSample {} => { id: "play_previous_random_sample", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::PlayPreviousRandomSample },
    AdjustSelectedBrowserRating { delta } => { id: "adjust_selected_browser_rating", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::AdjustSelectedBrowserRating { delta: 1 } },
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
    ReverseWaveformSelection {} => { id: "reverse_waveform_selection", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ReverseWaveformSelection },
    FadeWaveformSelectionLeftToRight {} => { id: "fade_waveform_selection_left_to_right", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FadeWaveformSelectionLeftToRight },
    FadeWaveformSelectionRightToLeft {} => { id: "fade_waveform_selection_right_to_left", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FadeWaveformSelectionRightToLeft },
    MuteWaveformSelection {} => { id: "mute_waveform_selection", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::MuteWaveformSelection },
    DeleteSelectedSliceMarkers {} => { id: "delete_selected_slice_markers", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::DeleteSelectedSliceMarkers },
    AlignWaveformStartToMarker {} => { id: "align_waveform_start_to_marker", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::AlignWaveformStartToMarker },
    DeleteLoadedWaveformSample {} => { id: "delete_loaded_waveform_sample", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::DeleteLoadedWaveformSample },
    SlideWaveformSelection { delta, fine } => { id: "slide_waveform_selection", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SlideWaveformSelection { delta: 1, fine: false } },
    ConfirmPrompt {} => { id: "confirm_prompt", surface: Prompt, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["prompt"], sample: NativeUiAction::ConfirmPrompt },
    CancelPrompt {} => { id: "cancel_prompt", surface: Prompt, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["prompt"], sample: NativeUiAction::CancelPrompt },
    CancelProgress {} => { id: "cancel_progress", surface: Prompt, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["progress"], sample: NativeUiAction::CancelProgress },
    ToggleHotkeyOverlay {} => { id: "toggle_hotkey_overlay", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleHotkeyOverlay },
    CopyStatusLog {} => { id: "copy_status_log", surface: Browser, effect: IoJob, coverage: [SemanticContract, RuntimeInput], fixtures: ["browser"], sample: NativeUiAction::CopyStatusLog },
    OpenFeedbackIssuePrompt {} => { id: "open_feedback_issue_prompt", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::OpenFeedbackIssuePrompt },
    MoveTrashedSamplesToFolder {} => { id: "move_trashed_samples_to_folder", surface: Browser, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::MoveTrashedSamplesToFolder },
    SetInputMonitoringEnabled { enabled } => { id: "set_input_monitoring_enabled", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetInputMonitoringEnabled { enabled: true } },
    SetAdvanceAfterRatingEnabled { enabled } => { id: "set_advance_after_rating_enabled", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAdvanceAfterRatingEnabled { enabled: true } },
    SetDestructiveYoloMode { enabled } => { id: "set_destructive_yolo_mode", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetDestructiveYoloMode { enabled: true } },
    SetInvertWaveformScroll { enabled } => { id: "set_invert_waveform_scroll", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetInvertWaveformScroll { enabled: true } },
    ToggleLoopPlayback {} => { id: "toggle_loop_playback", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::ToggleLoopPlayback },
    ToggleLoopLock {} => { id: "toggle_loop_lock", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleLoopLock },
    SetWaveformChannelView { stereo } => { id: "set_waveform_channel_view", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformChannelView { stereo: true } },
    SetNormalizedAuditionEnabled { enabled } => { id: "set_normalized_audition_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetNormalizedAuditionEnabled { enabled: true } },
    SetBpmSnapEnabled { enabled } => { id: "set_bpm_snap_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetBpmSnapEnabled { enabled: true } },
    AdjustWaveformBpm { delta } => { id: "adjust_waveform_bpm", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::AdjustWaveformBpm { delta: 1 } },
    SetWaveformBpmValue { value_tenths } => { id: "set_waveform_bpm_value", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformBpmValue { value_tenths: 1280 } },
    SetTransientSnapEnabled { enabled } => { id: "set_transient_snap_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetTransientSnapEnabled { enabled: true } },
    SetTransientMarkersEnabled { enabled } => { id: "set_transient_markers_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetTransientMarkersEnabled { enabled: true } },
    ToggleTransientMarkers {} => { id: "toggle_transient_markers", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleTransientMarkers },
    ToggleBpmSnap {} => { id: "toggle_bpm_snap", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleBpmSnap },
    SetSliceModeEnabled { enabled } => { id: "set_slice_mode_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetSliceModeEnabled { enabled: true } },
    SetVolume { value_milli } => { id: "set_volume", surface: Transport, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["transport"], sample: NativeUiAction::SetVolume { value_milli: 750 } },
    CommitVolumeSetting {} => { id: "commit_volume_setting", surface: Transport, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["transport"], sample: NativeUiAction::CommitVolumeSetting },
    SeekWaveformPrecise { position_nanos } => { id: "seek_waveform_precise", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SeekWaveformPrecise { position_nanos: 450_000_000 } },
    SetWaveformCursorPrecise { position_nanos } => { id: "set_waveform_cursor_precise", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformCursorPrecise { position_nanos: 450_000_000 } },
    SeekWaveform { position_milli } => { id: "seek_waveform", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SeekWaveform { position_milli: 450 } },
    SetWaveformCursor { position_milli } => { id: "set_waveform_cursor", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformCursor { position_milli: 450 } },
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
    SetWaveformViewCenter { center_micros } => { id: "set_waveform_view_center", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformViewCenter { center_micros: 500_000 } },
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
