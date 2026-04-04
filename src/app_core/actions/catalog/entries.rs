//! Stable GUI action catalog entries and lookup helpers.

use super::super::NativeUiAction;
use super::{
    GuiActionCatalogEntry, GuiActionKind, GuiCoverageLayer, GuiDispatchPolicy, GuiEffectClass,
    GuiHistoryPolicy, GuiSurface,
};

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
                    dispatch_policy: gui_dispatch_policy(GuiActionKind::$kind),
                    history_policy: gui_history_policy(
                        GuiActionKind::$kind,
                        GuiEffectClass::$effect,
                    ),
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
        NativeUiAction::$kind { $($field: _,)+ .. }
    };
}

const fn gui_dispatch_policy(kind: GuiActionKind) -> GuiDispatchPolicy {
    match kind {
        GuiActionKind::BeginWaveformSelectionShift
        | GuiActionKind::BeginWaveformEditSelectionShift => GuiDispatchPolicy::RuntimeInternal,
        _ => GuiDispatchPolicy::Public,
    }
}

const fn gui_history_policy(kind: GuiActionKind, _effect: GuiEffectClass) -> GuiHistoryPolicy {
    match kind {
        GuiActionKind::FocusSourceRow
        | GuiActionKind::SelectSourceRow
        | GuiActionKind::MoveSourceFocus
        | GuiActionKind::FocusFolderRow
        | GuiActionKind::ActivateFolderRow
        | GuiActionKind::ToggleShowAllFolders
        | GuiActionKind::ToggleFolderFlattenedView
        | GuiActionKind::ToggleFocusedFolderSelection
        | GuiActionKind::MoveFolderFocus
        | GuiActionKind::MoveBrowserFocus
        | GuiActionKind::FocusBrowserRow
        | GuiActionKind::CommitFocusedBrowserRow
        | GuiActionKind::ToggleBrowserRowSelection
        | GuiActionKind::ExtendBrowserSelectionToRow
        | GuiActionKind::AddRangeBrowserSelection
        | GuiActionKind::ExtendBrowserSelectionFromFocus
        | GuiActionKind::AddRangeBrowserSelectionFromFocus
        | GuiActionKind::ToggleFocusedBrowserRowSelection
        | GuiActionKind::SelectAllBrowserRows
        | GuiActionKind::FinishWaveformSelectionDrag
        | GuiActionKind::FinishWaveformCircularSlide
        | GuiActionKind::FinishWaveformSelectionRangeDrag
        | GuiActionKind::FinishWaveformSelectionSmartScaleDrag
        | GuiActionKind::FinishWaveformEditSelectionDrag
        | GuiActionKind::FinishWaveformEditFadeDrag
        | GuiActionKind::ClearWaveformSelection
        | GuiActionKind::ClearWaveformEditSelection
        | GuiActionKind::ClearWaveformSelections
        | GuiActionKind::SlideWaveformSelection
        | GuiActionKind::TagBrowserSelection
        | GuiActionKind::AdjustSelectedBrowserRating
        | GuiActionKind::DeleteFocusedFolder => GuiHistoryPolicy::Immediate,
        GuiActionKind::NormalizeFocusedBrowserSample
        | GuiActionKind::SaveWaveformSelectionToBrowser
        | GuiActionKind::SaveWaveformSelectionToBrowserWithKeep2 => GuiHistoryPolicy::Deferred,
        _ => GuiHistoryPolicy::None,
    }
}

gui_action_catalog!(
    SelectColumn { index } => { id: "select_column", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::SelectColumn { index: 1 } },
    MoveColumn { delta } => { id: "move_column", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::MoveColumn { delta: 1 } },
    ToggleTransport {} => { id: "toggle_transport", surface: Transport, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["transport"], sample: NativeUiAction::ToggleTransport },
    PlayCompareAnchor {} => { id: "play_compare_anchor", surface: Transport, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["transport"], sample: NativeUiAction::PlayCompareAnchor },
    PlayFromStart {} => { id: "play_from_start", surface: Transport, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["transport"], sample: NativeUiAction::PlayFromStart },
    PlayFromCurrentPlayhead {} => { id: "play_from_current_playhead", surface: Transport, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["transport"], sample: NativeUiAction::PlayFromCurrentPlayhead },
    PlayFromWaveformCursor {} => { id: "play_from_waveform_cursor", surface: Transport, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["transport"], sample: NativeUiAction::PlayFromWaveformCursor },
    PlayWaveformAtPrecise { position_nanos } => { id: "play_waveform_at_precise", surface: Transport, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::PlayWaveformAtPrecise { position_nanos: 450_000_000 } },
    HandleEscape {} => { id: "handle_escape", surface: Prompt, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["prompt"], sample: NativeUiAction::HandleEscape },
    FocusBrowserPanel {} => { id: "focus_browser_panel", surface: Browser, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["browser"], sample: NativeUiAction::FocusBrowserPanel },
    FocusSourcesPanel {} => { id: "focus_sources_panel", surface: Sources, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::FocusSourcesPanel },
    FocusWaveformPanel {} => { id: "focus_waveform_panel", surface: Waveform, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["waveform"], sample: NativeUiAction::FocusWaveformPanel },
    FocusFolderPanel { pane } => { id: "focus_folder_panel", surface: Sources, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::FocusFolderPanel { pane: None } },
    FocusLoadedSampleInBrowser {} => { id: "focus_loaded_sample_in_browser", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusLoadedSampleInBrowser },
    FocusBrowserSearch {} => { id: "focus_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "search"], sample: NativeUiAction::FocusBrowserSearch },
    BlurBrowserSearch {} => { id: "blur_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "search"], sample: NativeUiAction::BlurBrowserSearch },
    OpenAddSourceDialog {} => { id: "open_add_source_dialog", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::OpenAddSourceDialog },
    OpenOptionsMenu {} => { id: "open_options_menu", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenOptionsMenu },
    CloseOptionsPanel {} => { id: "close_options_panel", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["options"], sample: NativeUiAction::CloseOptionsPanel },
    ShowOptionsOverview {} => { id: "show_options_overview", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::ShowOptionsOverview },
    PickTrashFolder {} => { id: "pick_trash_folder", surface: Options, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::PickTrashFolder },
    OpenTrashFolder {} => { id: "open_trash_folder", surface: Options, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenTrashFolder },
    OpenAudioOutputHostPicker {} => { id: "open_audio_output_host_picker", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenAudioOutputHostPicker },
    OpenAudioOutputDevicePicker {} => { id: "open_audio_output_device_picker", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenAudioOutputDevicePicker },
    OpenAudioOutputSampleRatePicker {} => { id: "open_audio_output_sample_rate_picker", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenAudioOutputSampleRatePicker },
    OpenAudioInputHostPicker {} => { id: "open_audio_input_host_picker", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenAudioInputHostPicker },
    OpenAudioInputDevicePicker {} => { id: "open_audio_input_device_picker", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenAudioInputDevicePicker },
    OpenAudioInputSampleRatePicker {} => { id: "open_audio_input_sample_rate_picker", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::OpenAudioInputSampleRatePicker },
    SetAudioOutputHost { host_id } => { id: "set_audio_output_host", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAudioOutputHost { host_id: Some(String::from("wasapi")) } },
    SetAudioOutputDevice { device_name } => { id: "set_audio_output_device", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAudioOutputDevice { device_name: Some(String::from("Built-in Output")) } },
    SetAudioOutputSampleRate { sample_rate } => { id: "set_audio_output_sample_rate", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAudioOutputSampleRate { sample_rate: Some(48_000) } },
    SetAudioInputHost { host_id } => { id: "set_audio_input_host", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAudioInputHost { host_id: Some(String::from("wasapi")) } },
    SetAudioInputDevice { device_name } => { id: "set_audio_input_device", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAudioInputDevice { device_name: Some(String::from("Built-in Input")) } },
    SetAudioInputSampleRate { sample_rate } => { id: "set_audio_input_sample_rate", surface: Options, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["options"], sample: NativeUiAction::SetAudioInputSampleRate { sample_rate: Some(48_000) } },
    FocusFolderSearch { pane } => { id: "focus_folder_search", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusFolderSearch { pane: None } },
    SetFolderSearch { pane, query } => { id: "set_folder_search", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::SetFolderSearch { pane: None, query: String::from("drums") } },
    ToggleShowAllFolders { pane } => { id: "toggle_show_all_folders", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ToggleShowAllFolders { pane: None } },
    ToggleFolderFlattenedView { pane } => { id: "toggle_folder_flattened_view", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ToggleFolderFlattenedView { pane: None } },
    FocusSourceRow { pane, index } => { id: "focus_source_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusSourceRow { pane: None, index: 0 } },
    SelectSourceRow { pane, index } => { id: "select_source_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::SelectSourceRow { pane: None, index: 0 } },
    MoveSourceFocus { delta } => { id: "move_source_focus", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::MoveSourceFocus { delta: 1 } },
    ReloadFocusedSourceRow {} => { id: "reload_focused_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ReloadFocusedSourceRow },
    HardSyncFocusedSourceRow {} => { id: "hard_sync_focused_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::HardSyncFocusedSourceRow },
    OpenFocusedSourceFolder {} => { id: "open_focused_source_folder", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::OpenFocusedSourceFolder },
    RemoveFocusedSourceRow {} => { id: "remove_focused_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveFocusedSourceRow },
    ReloadSourceRow { pane, index } => { id: "reload_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ReloadSourceRow { pane: None, index: 0 } },
    HardSyncSourceRow { pane, index } => { id: "hard_sync_source_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::HardSyncSourceRow { pane: None, index: 0 } },
    OpenSourceFolderRow { pane, index } => { id: "open_source_folder_row", surface: Sources, effect: IoJob, coverage: [SemanticContract, RuntimeInput], fixtures: ["sources"], sample: NativeUiAction::OpenSourceFolderRow { pane: None, index: 0 } },
    RemoveSourceRow { pane, index } => { id: "remove_source_row", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::RemoveSourceRow { pane: None, index: 0 } },
    FocusFolderRow { pane, index } => { id: "focus_folder_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusFolderRow { pane: None, index: 0 } },
    ActivateFolderRow { pane, index } => { id: "activate_folder_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ActivateFolderRow { pane: None, index: 0 } },
    ToggleFolderRowExpanded { pane, index } => { id: "toggle_folder_row_expanded", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ToggleFolderRowExpanded { pane: None, index: 0 } },
    ExpandFocusedFolder {} => { id: "expand_focused_folder", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ExpandFocusedFolder },
    CollapseFocusedFolder {} => { id: "collapse_focused_folder", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::CollapseFocusedFolder },
    ToggleFocusedFolderSelection {} => { id: "toggle_focused_folder_selection", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ToggleFocusedFolderSelection },
    MoveFolderFocus { delta } => { id: "move_folder_focus", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::MoveFolderFocus { delta: 1 } },
    StartNewFolder {} => { id: "start_new_folder", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartNewFolder },
    StartNewFolderAtFolderRow { pane, index } => { id: "start_new_folder_at_folder_row", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartNewFolderAtFolderRow { pane: None, index: 0 } },
    StartNewFolderAtRoot {} => { id: "start_new_folder_at_root", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartNewFolderAtRoot },
    FocusFolderCreateInput {} => { id: "focus_folder_create_input", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::FocusFolderCreateInput },
    SetFolderCreateInput { value } => { id: "set_folder_create_input", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::SetFolderCreateInput { value: String::from("drums") } },
    ConfirmFolderCreate {} => { id: "confirm_folder_create", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ConfirmFolderCreate },
    CancelFolderCreate {} => { id: "cancel_folder_create", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::CancelFolderCreate },
    StartFolderRename {} => { id: "start_folder_rename", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::StartFolderRename },
    DeleteFocusedFolder {} => { id: "delete_focused_folder", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::DeleteFocusedFolder },
    RestoreRetainedFolderDeletes {} => { id: "restore_retained_folder_deletes", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources", "prompt"], sample: NativeUiAction::RestoreRetainedFolderDeletes },
    PurgeRetainedFolderDeletes {} => { id: "purge_retained_folder_deletes", surface: Sources, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources", "prompt"], sample: NativeUiAction::PurgeRetainedFolderDeletes },
    ClearFolderDeleteRecoveryLog {} => { id: "clear_folder_delete_recovery_log", surface: Sources, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["sources"], sample: NativeUiAction::ClearFolderDeleteRecoveryLog },
    MoveBrowserFocus { delta } => { id: "move_browser_focus", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::MoveBrowserFocus { delta: 1 } },
    SetBrowserViewStart { visible_row } => { id: "set_browser_view_start", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::SetBrowserViewStart { visible_row: 4 } },
    FocusBrowserRow { visible_row } => { id: "focus_browser_row", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusBrowserRow { visible_row: 2 } },
    SetCompareAnchorFromFocusedBrowserSample {} => { id: "set_compare_anchor_from_focused_browser_sample", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "transport"], sample: NativeUiAction::SetCompareAnchorFromFocusedBrowserSample },
    CommitFocusedBrowserRow {} => { id: "commit_focused_browser_row", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser"], sample: NativeUiAction::CommitFocusedBrowserRow },
    SaveWaveformSelectionToBrowser {} => { id: "save_waveform_selection_to_browser", surface: Waveform, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform", "browser"], sample: NativeUiAction::SaveWaveformSelectionToBrowser },
    SaveWaveformSelectionToBrowserWithKeep2 {} => { id: "save_waveform_selection_to_browser_with_keep2", surface: Waveform, effect: IoJob, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform", "browser"], sample: NativeUiAction::SaveWaveformSelectionToBrowserWithKeep2 },
    CommitWaveformEditFades {} => { id: "commit_waveform_edit_fades", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::CommitWaveformEditFades },
    DetectWaveformSilenceSlices {} => { id: "detect_waveform_silence_slices", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::DetectWaveformSilenceSlices },
    DetectWaveformExactDuplicateSlices {} => { id: "detect_waveform_exact_duplicate_slices", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::DetectWaveformExactDuplicateSlices },
    CleanWaveformExactDuplicateSlices {} => { id: "clean_waveform_exact_duplicate_slices", surface: Waveform, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::CleanWaveformExactDuplicateSlices },
    ToggleBrowserRowSelection { visible_row } => { id: "toggle_browser_row_selection", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserRowSelection { visible_row: 2 } },
    StartBrowserSampleDrag { visible_row, pointer_x, pointer_y } => { id: "start_browser_sample_drag", surface: Browser, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "sources"], sample: NativeUiAction::StartBrowserSampleDrag { visible_row: 2, pointer_x: 120, pointer_y: 80 } },
    UpdateBrowserSampleDrag { pointer_x, pointer_y, hovered_folder_pane, hovered_folder_row, over_folder_panel, shift_down, alt_down } => { id: "update_browser_sample_drag", surface: Browser, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "sources"], sample: NativeUiAction::UpdateBrowserSampleDrag { pointer_x: 180, pointer_y: 120, hovered_folder_pane: None, hovered_folder_row: Some(1), over_folder_panel: None, shift_down: false, alt_down: false } },
    FinishBrowserSampleDrag {} => { id: "finish_browser_sample_drag", surface: Browser, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser", "sources"], sample: NativeUiAction::FinishBrowserSampleDrag },
    ExtendBrowserSelectionToRow { visible_row } => { id: "extend_browser_selection_to_row", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ExtendBrowserSelectionToRow { visible_row: 3 } },
    AddRangeBrowserSelection { visible_row } => { id: "add_range_browser_selection", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::AddRangeBrowserSelection { visible_row: 3 } },
    ExtendBrowserSelectionFromFocus { delta } => { id: "extend_browser_selection_from_focus", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ExtendBrowserSelectionFromFocus { delta: 1 } },
    AddRangeBrowserSelectionFromFocus { delta } => { id: "add_range_browser_selection_from_focus", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::AddRangeBrowserSelectionFromFocus { delta: 1 } },
    ToggleFocusedBrowserRowSelection {} => { id: "toggle_focused_browser_row_selection", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleFocusedBrowserRowSelection },
    SelectAllBrowserRows {} => { id: "select_all_browser_rows", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::SelectAllBrowserRows },
    SetBrowserSearch { query } => { id: "set_browser_search", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser", "search"], sample: NativeUiAction::SetBrowserSearch { query: String::from("kick") } },
    ToggleBrowserRatingFilter { level, invert } => { id: "toggle_browser_rating_filter", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserRatingFilter { level: 3, invert: false } },
    ToggleBrowserPlaybackAgeFilter { bucket, invert } => { id: "toggle_browser_playback_age_filter", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserPlaybackAgeFilter { bucket: radiant::app::PlaybackAgeFilterChip::OlderThanWeek, invert: false } },
    ToggleBrowserSampleMark {} => { id: "toggle_browser_sample_mark", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserSampleMark },
    ToggleBrowserMarkedFilter {} => { id: "toggle_browser_marked_filter", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserMarkedFilter },
    ToggleRandomNavigationMode {} => { id: "toggle_random_navigation_mode", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleRandomNavigationMode },
    ToggleBrowserDuplicateCleanupMode {} => { id: "toggle_browser_duplicate_cleanup_mode", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserDuplicateCleanupMode },
    FocusPreviousBrowserHistory {} => { id: "focus_previous_browser_history", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusPreviousBrowserHistory },
    FocusNextBrowserHistory {} => { id: "focus_next_browser_history", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::FocusNextBrowserHistory },
    ToggleFindSimilarFocusedSample {} => { id: "toggle_find_similar_focused_sample", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleFindSimilarFocusedSample },
    ToggleBrowserDuplicateCleanupKeep { visible_row } => { id: "toggle_browser_duplicate_cleanup_keep", surface: Browser, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ToggleBrowserDuplicateCleanupKeep { visible_row: 2 } },
    ConfirmBrowserDuplicateCleanup {} => { id: "confirm_browser_duplicate_cleanup", surface: Browser, effect: Destructive, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["browser"], sample: NativeUiAction::ConfirmBrowserDuplicateCleanup },
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
    CopySelectionToClipboard {} => { id: "copy_selection_to_clipboard", surface: Browser, effect: IoJob, coverage: [SemanticContract, RuntimeInput], fixtures: ["browser", "waveform"], sample: NativeUiAction::CopySelectionToClipboard },
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
    SetRelativeBpmGridEnabled { enabled } => { id: "set_relative_bpm_grid_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetRelativeBpmGridEnabled { enabled: true } },
    AdjustWaveformBpm { delta } => { id: "adjust_waveform_bpm", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::AdjustWaveformBpm { delta: 1 } },
    SetWaveformBpmValue { value_tenths } => { id: "set_waveform_bpm_value", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformBpmValue { value_tenths: 1280 } },
    SetTransientSnapEnabled { enabled } => { id: "set_transient_snap_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetTransientSnapEnabled { enabled: true } },
    SetTransientMarkersEnabled { enabled } => { id: "set_transient_markers_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetTransientMarkersEnabled { enabled: true } },
    ToggleTransientMarkers {} => { id: "toggle_transient_markers", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleTransientMarkers },
    ToggleBpmSnap {} => { id: "toggle_bpm_snap", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleBpmSnap },
    SetSliceModeEnabled { enabled } => { id: "set_slice_mode_enabled", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetSliceModeEnabled { enabled: true } },
    ToggleWaveformSliceSelection { index } => { id: "toggle_waveform_slice_selection", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleWaveformSliceSelection { index: 0 } },
    AuditionWaveformDuplicateSlice { index } => { id: "audition_waveform_duplicate_slice", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::AuditionWaveformDuplicateSlice { index: 0 } },
    ToggleWaveformDuplicateSliceExemption { index } => { id: "toggle_waveform_duplicate_slice_exemption", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleWaveformDuplicateSliceExemption { index: 0 } },
    MoveWaveformSliceFocus { delta } => { id: "move_waveform_slice_focus", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::MoveWaveformSliceFocus { delta: 1 } },
    ToggleFocusedWaveformSliceExportMark {} => { id: "toggle_focused_waveform_slice_export_mark", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ToggleFocusedWaveformSliceExportMark },
    SetVolume { value_milli } => { id: "set_volume", surface: Transport, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["transport"], sample: NativeUiAction::SetVolume { value_milli: 750 } },
    CommitVolumeSetting {} => { id: "commit_volume_setting", surface: Transport, effect: StateOnly, coverage: [SemanticContract, RuntimeInput], fixtures: ["transport"], sample: NativeUiAction::CommitVolumeSetting },
    SeekWaveformPrecise { position_nanos } => { id: "seek_waveform_precise", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SeekWaveformPrecise { position_nanos: 450_000_000 } },
    SetWaveformCursorPrecise { position_nanos } => { id: "set_waveform_cursor_precise", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformCursorPrecise { position_nanos: 450_000_000 } },
    SeekWaveform { position_milli } => { id: "seek_waveform", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SeekWaveform { position_milli: 450 } },
    SetWaveformCursor { position_milli } => { id: "set_waveform_cursor", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformCursor { position_milli: 450 } },
    BeginWaveformCircularSlide { anchor_micros } => { id: "begin_waveform_circular_slide", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::BeginWaveformCircularSlide { anchor_micros: 125_000 } },
    UpdateWaveformCircularSlide { position_micros } => { id: "update_waveform_circular_slide", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::UpdateWaveformCircularSlide { position_micros: 375_000 } },
    FinishWaveformCircularSlide {} => { id: "finish_waveform_circular_slide", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformCircularSlide },
    BeginWaveformSelectionAt { anchor_micros } => { id: "begin_waveform_selection_at", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::BeginWaveformSelectionAt { anchor_micros: 125_000 } },
    SetWaveformSelectionRange { start_micros, end_micros, snap_override, preserve_view_edge } => { id: "set_waveform_selection_range", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformSelectionRange { start_micros: 100_000, end_micros: 300_000, snap_override: false, preserve_view_edge: false } },
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
    UpdateWaveformSelectionDrag { pointer_x, pointer_y, hovered_folder_pane, hovered_folder_row, over_folder_panel, over_browser_list, shift_down, alt_down } => { id: "update_waveform_selection_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::UpdateWaveformSelectionDrag { pointer_x: 180, pointer_y: 80, hovered_folder_pane: None, hovered_folder_row: None, over_folder_panel: None, over_browser_list: false, shift_down: false, alt_down: false } },
    FinishWaveformSelectionDrag {} => { id: "finish_waveform_selection_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformSelectionDrag },
    FinishWaveformSelectionRangeDrag {} => { id: "finish_waveform_selection_range_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformSelectionRangeDrag },
    FinishWaveformSelectionSmartScaleDrag {} => { id: "finish_waveform_selection_smart_scale_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformSelectionSmartScaleDrag },
    BeginWaveformSelectionShift { pointer_micros, start_micros, end_micros } => { id: "begin_waveform_selection_shift", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::BeginWaveformSelectionShift { pointer_micros: 200_000, start_micros: 100_000, end_micros: 300_000 } },
    BeginWaveformEditSelectionShift { pointer_micros, start_micros, end_micros } => { id: "begin_waveform_edit_selection_shift", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::BeginWaveformEditSelectionShift { pointer_micros: 200_000, start_micros: 100_000, end_micros: 300_000 } },
    FinishWaveformEditSelectionDrag {} => { id: "finish_waveform_edit_selection_drag", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::FinishWaveformEditSelectionDrag },
    ClearWaveformSelection {} => { id: "clear_waveform_selection", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ClearWaveformSelection },
    ClearWaveformEditSelection {} => { id: "clear_waveform_edit_selection", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::ClearWaveformEditSelection },
    ClearWaveformSelections {} => { id: "clear_waveform_selections", surface: Waveform, effect: Projection, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot, DesktopAiv], fixtures: ["waveform"], sample: NativeUiAction::ClearWaveformSelections },
    SetWaveformViewCenter { center_micros, center_nanos } => { id: "set_waveform_view_center", surface: Waveform, effect: RuntimeMotion, coverage: [SemanticContract, RuntimeInput, ProjectionSnapshot], fixtures: ["waveform"], sample: NativeUiAction::SetWaveformViewCenter { center_micros: 500_000, center_nanos: None } },
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
