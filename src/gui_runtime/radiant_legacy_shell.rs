use super::{NativeRunOptions, NativeRunReport, NativeRuntimeArtifacts, WindowIconRgba};
use crate::app::{
    controller::ui::hotkeys::{self, KeyPress},
    state::FocusContext,
};
use crate::app_core::actions::{
    NativeAppBridge, NativeAppModel, NativeBrowserTagTarget as BrowserTagTarget,
    NativeFrameBuildResult, NativeGuiAutomationSnapshot, NativeMotionModel, NativeUiAction,
    NativeUiAction as UiAction,
};
use radiant::compat::legacy_shell as compat;
use std::sync::Arc;

/// Converts app-level Vello launch options into the hosted `radiant` representation.
///
/// Mapping is intentionally field-for-field to preserve behavior and avoid
/// hidden launch-time mutations.
impl From<NativeRunOptions> for radiant::compat::legacy_shell::NativeRunOptions {
    fn from(value: NativeRunOptions) -> Self {
        Self {
            title: value.title,
            inner_size: value.inner_size,
            min_inner_size: value.min_inner_size,
            maximized: value.maximized,
            decorations: value.decorations,
            icon: value.icon.map(Into::into),
            target_fps: value.target_fps,
        }
    }
}

/// Converts app-level icon payloads into `radiant` host icon payloads.
///
/// All pixel bytes are forwarded unchanged; callers remain responsible for
/// supplying valid RGBA data and matching dimensions.
impl From<WindowIconRgba> for radiant::compat::legacy_shell::WindowIconRgba {
    fn from(value: WindowIconRgba) -> Self {
        Self {
            rgba: value.rgba,
            width: value.width,
            height: value.height,
        }
    }
}

struct CompatNativeAppBridge<B> {
    inner: B,
}

impl<B> CompatNativeAppBridge<B> {
    fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: NativeAppBridge> radiant::compat::legacy_shell::NativeAppBridge
    for CompatNativeAppBridge<B>
{
    fn project_model(&mut self) -> Arc<radiant::compat::legacy_shell::AppModel> {
        let model = self.inner.project_model();
        Arc::new(model.as_ref().into())
    }

    fn pull_model(&mut self) -> radiant::compat::legacy_shell::AppModel {
        self.inner.pull_model().into()
    }

    fn pull_model_arc(&mut self) -> Arc<radiant::compat::legacy_shell::AppModel> {
        let model = self.inner.pull_model_arc();
        Arc::new(model.as_ref().into())
    }

    fn project_motion_model(&mut self) -> Option<radiant::compat::legacy_shell::NativeMotionModel> {
        self.inner
            .project_motion_model()
            .map(NativeMotionModel::into)
    }

    fn take_dirty_segments(&mut self) -> radiant::compat::legacy_shell::DirtySegments {
        self.inner.take_dirty_segments().into()
    }

    fn take_segment_revisions(&mut self) -> radiant::compat::legacy_shell::SegmentRevisions {
        self.inner.take_segment_revisions().into()
    }

    fn resolve_hotkey_press(
        &mut self,
        pending_chord: Option<radiant::compat::legacy_shell::KeyPress>,
        press: radiant::compat::legacy_shell::KeyPress,
        focus: radiant::compat::legacy_shell::FocusContextModel,
    ) -> radiant::compat::legacy_shell::HotkeyResolution {
        let resolution = hotkeys::resolve_hotkey_press(
            pending_chord.map(keypress_from_radiant),
            keypress_from_radiant(press),
            focus_context_from_radiant(focus),
        );
        radiant::compat::legacy_shell::HotkeyResolution {
            action: resolution.action.map(Into::into),
            handled: resolution.handled,
            pending_chord: resolution.pending_chord.map(keypress_to_radiant),
        }
    }

    fn reduce_action(&mut self, action: radiant::compat::legacy_shell::UiAction) {
        self.inner.reduce_action(NativeUiAction::from(action));
    }

    fn take_last_action_handled(&mut self) -> Option<bool> {
        self.inner.take_last_action_handled()
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.inner.install_repaint_signal(signal);
    }

    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, hwnd: isize) {
        self.inner.set_external_drag_hwnd(hwnd);
    }

    #[cfg(target_os = "windows")]
    fn maybe_launch_external_drag(&mut self, pointer_outside: bool, pointer_left: bool) -> bool {
        self.inner
            .maybe_launch_external_drag(pointer_outside, pointer_left)
    }

    fn observe_frame_result(&mut self, result: radiant::compat::legacy_shell::FrameBuildResult) {
        self.inner
            .observe_frame_result(NativeFrameBuildResult::from(result));
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.inner
            .on_runtime_exit()
            .and_then(|artifact| serde_json::to_value(artifact).ok())
    }
}

fn native_run_report_from_radiant(
    report: radiant::compat::legacy_shell::NativeRunReport,
) -> NativeRunReport {
    NativeRunReport {
        artifacts: NativeRuntimeArtifacts {
            startup_timing: report.artifacts.startup_timing,
            shutdown_timing: report
                .artifacts
                .shutdown_timing
                .and_then(|value| serde_json::from_value(value).ok()),
        },
        result: report.result,
    }
}

fn focus_context_from_radiant(
    focus: radiant::compat::legacy_shell::FocusContextModel,
) -> FocusContext {
    match focus {
        radiant::compat::legacy_shell::FocusContextModel::None => FocusContext::None,
        radiant::compat::legacy_shell::FocusContextModel::Timeline => FocusContext::Waveform,
        radiant::compat::legacy_shell::FocusContextModel::ContentList => {
            FocusContext::SampleBrowser
        }
        radiant::compat::legacy_shell::FocusContextModel::NavigationTree => {
            FocusContext::SourceFolders
        }
        radiant::compat::legacy_shell::FocusContextModel::NavigationList => {
            FocusContext::SourcesList
        }
    }
}

fn keypress_from_radiant(press: radiant::compat::legacy_shell::KeyPress) -> KeyPress {
    KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

fn keypress_to_radiant(press: KeyPress) -> radiant::compat::legacy_shell::KeyPress {
    radiant::compat::legacy_shell::KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

impl From<compat::BrowserTriageTarget> for BrowserTagTarget {
    fn from(value: compat::BrowserTriageTarget) -> Self {
        match value {
            compat::BrowserTriageTarget::Negative => Self::Trash,
            compat::BrowserTriageTarget::Neutral => Self::Neutral,
            compat::BrowserTriageTarget::Positive => Self::Keep,
        }
    }
}

impl From<BrowserTagTarget> for compat::BrowserTriageTarget {
    fn from(value: BrowserTagTarget) -> Self {
        match value {
            BrowserTagTarget::Trash => Self::Negative,
            BrowserTagTarget::Neutral => Self::Neutral,
            BrowserTagTarget::Keep => Self::Positive,
        }
    }
}

impl From<compat::UiAction> for UiAction {
    fn from(value: compat::UiAction) -> Self {
        match value {
            compat::UiAction::SelectColumn { index } => Self::SelectColumn { index: index },
            compat::UiAction::MoveColumn { delta } => Self::MoveColumn { delta: delta },
            compat::UiAction::ToggleTransport => Self::ToggleTransport,
            compat::UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            compat::UiAction::PlayFromStart => Self::PlayFromStart,
            compat::UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            compat::UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            compat::UiAction::PlayWaveformAtPrecise { position_nanos } => {
                Self::PlayWaveformAtPrecise {
                    position_nanos: position_nanos,
                }
            }
            compat::UiAction::HandleEscape => Self::HandleEscape,
            compat::UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            compat::UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            compat::UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            compat::UiAction::FocusFolderPanel { pane } => Self::FocusFolderPanel {
                pane: pane.map(Into::into),
            },
            compat::UiAction::FocusLoadedContentInList => Self::FocusLoadedSampleInBrowser,
            compat::UiAction::FocusBrowserSearch => Self::FocusBrowserSearch,
            compat::UiAction::BlurBrowserSearch => Self::BlurBrowserSearch,
            compat::UiAction::OpenAddSourceDialog => Self::OpenAddSourceDialog,
            compat::UiAction::OpenOptionsMenu => Self::OpenOptionsMenu,
            compat::UiAction::CloseOptionsPanel => Self::CloseOptionsPanel,
            compat::UiAction::PickTrashFolder => Self::PickTrashFolder,
            compat::UiAction::OpenTrashFolder => Self::OpenTrashFolder,
            compat::UiAction::EditDefaultIdentifier => Self::EditDefaultIdentifier,
            compat::UiAction::ShowOptionsOverview => Self::ShowOptionsOverview,
            compat::UiAction::OpenPrimaryGroupPicker => Self::OpenAudioOutputHostPicker,
            compat::UiAction::OpenPrimaryItemPicker => Self::OpenAudioOutputDevicePicker,
            compat::UiAction::OpenPrimaryNumberPicker => Self::OpenAudioOutputSampleRatePicker,
            compat::UiAction::OpenSecondaryGroupPicker => Self::OpenAudioInputHostPicker,
            compat::UiAction::OpenSecondaryItemPicker => Self::OpenAudioInputDevicePicker,
            compat::UiAction::OpenSecondaryNumberPicker => Self::OpenAudioInputSampleRatePicker,
            compat::UiAction::SetPrimaryGroup { group_id } => {
                Self::SetAudioOutputHost { host_id: group_id }
            }
            compat::UiAction::SetPrimaryItem { item_name } => Self::SetAudioOutputDevice {
                device_name: item_name,
            },
            compat::UiAction::SetPrimaryNumber { value } => {
                Self::SetAudioOutputSampleRate { sample_rate: value }
            }
            compat::UiAction::SetSecondaryGroup { group_id } => {
                Self::SetAudioInputHost { host_id: group_id }
            }
            compat::UiAction::SetSecondaryItem { item_name } => Self::SetAudioInputDevice {
                device_name: item_name,
            },
            compat::UiAction::SetSecondaryNumber { value } => {
                Self::SetAudioInputSampleRate { sample_rate: value }
            }
            compat::UiAction::FocusFolderSearch { pane } => Self::FocusFolderSearch {
                pane: pane.map(Into::into),
            },
            compat::UiAction::SetFolderSearch { pane, query } => Self::SetFolderSearch {
                pane: pane.map(Into::into),
                query: query,
            },
            compat::UiAction::ToggleShowAllFolders { pane } => Self::ToggleShowAllFolders {
                pane: pane.map(Into::into),
            },
            compat::UiAction::ToggleFolderFlattenedView { pane } => {
                Self::ToggleFolderFlattenedView {
                    pane: pane.map(Into::into),
                }
            }
            compat::UiAction::FocusSourceRow { pane, index } => Self::FocusSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::SelectSourceRow { pane, index } => Self::SelectSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta: delta },
            compat::UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            compat::UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            compat::UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            compat::UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            compat::UiAction::ReloadSourceRow { pane, index } => Self::ReloadSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::HardSyncSourceRow { pane, index } => Self::HardSyncSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::OpenSourceFolderRow { pane, index } => Self::OpenSourceFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::RemoveSourceRow { pane, index } => Self::RemoveSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::FocusFolderRow { pane, index } => Self::FocusFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::ActivateFolderRow { pane, index } => Self::ActivateFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::ToggleFolderRowExpanded { pane, index } => {
                Self::ToggleFolderRowExpanded {
                    pane: pane.map(Into::into),
                    index: index,
                }
            }
            compat::UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            compat::UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            compat::UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            compat::UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta: delta },
            compat::UiAction::StartNewFolder => Self::StartNewFolder,
            compat::UiAction::StartNewFolderAtFolderRow { pane, index } => {
                Self::StartNewFolderAtFolderRow {
                    pane: pane.map(Into::into),
                    index: index,
                }
            }
            compat::UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            compat::UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            compat::UiAction::SetFolderCreateInput { value } => {
                Self::SetFolderCreateInput { value: value }
            }
            compat::UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            compat::UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            compat::UiAction::StartFolderRename => Self::StartFolderRename,
            compat::UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            compat::UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            compat::UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            compat::UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            compat::UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta: delta },
            compat::UiAction::SetBrowserViewStart { visible_row } => Self::SetBrowserViewStart {
                visible_row: visible_row,
            },
            compat::UiAction::FocusBrowserRow { visible_row } => Self::FocusBrowserRow {
                visible_row: visible_row,
            },
            compat::UiAction::SetCompareAnchorFromFocusedContent => {
                Self::SetCompareAnchorFromFocusedBrowserSample
            }
            compat::UiAction::CommitFocusedBrowserRow => Self::CommitFocusedBrowserRow,
            compat::UiAction::SaveWaveformSelectionToBrowser => {
                Self::SaveWaveformSelectionToBrowser
            }
            compat::UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
                Self::SaveWaveformSelectionToBrowserWithKeep2
            }
            compat::UiAction::CommitWaveformEditFades => Self::CommitWaveformEditFades,
            compat::UiAction::DetectWaveformSilenceSlices => Self::DetectWaveformSilenceSlices,
            compat::UiAction::DetectWaveformExactDuplicateSlices => {
                Self::DetectWaveformExactDuplicateSlices
            }
            compat::UiAction::CleanWaveformExactDuplicateSlices => {
                Self::CleanWaveformExactDuplicateSlices
            }
            compat::UiAction::ToggleBrowserRowSelection { visible_row } => {
                Self::ToggleBrowserRowSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::StartContentItemDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartBrowserSampleDrag {
                visible_row: visible_row,
                pointer_x: pointer_x,
                pointer_y: pointer_y,
            },
            compat::UiAction::UpdateContentItemDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            } => Self::UpdateBrowserSampleDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                shift_down: shift_down,
                alt_down: alt_down,
            },
            compat::UiAction::FinishContentItemDrag => Self::FinishBrowserSampleDrag,
            compat::UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeBrowserSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta: delta }
            }
            compat::UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta: delta }
            }
            compat::UiAction::ToggleFocusedBrowserRowSelection => {
                Self::ToggleFocusedBrowserRowSelection
            }
            compat::UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            compat::UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query: query },
            compat::UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter {
                    level: level,
                    invert: invert,
                }
            }
            compat::UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter {
                    bucket: bucket.into(),
                    invert: invert,
                }
            }
            compat::UiAction::ToggleContentMark => Self::ToggleBrowserSampleMark,
            compat::UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            compat::UiAction::ToggleBrowserDerivedLabelFilter { invert } => {
                Self::ToggleBrowserTagNamedFilter { invert: invert }
            }
            compat::UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            compat::UiAction::ToggleBrowserPillEditor => Self::ToggleBrowserTagSidebar,
            compat::UiAction::ToggleBrowserPillEditorPrimaryAction => {
                Self::ToggleBrowserTagSidebarAutoRename
            }
            compat::UiAction::ToggleBrowserDuplicateCleanupMode => {
                Self::ToggleBrowserDuplicateCleanupMode
            }
            compat::UiAction::FocusPreviousBrowserHistory => Self::FocusPreviousBrowserHistory,
            compat::UiAction::FocusNextBrowserHistory => Self::FocusNextBrowserHistory,
            compat::UiAction::ToggleFindSimilarFocusedContent => {
                Self::ToggleFindSimilarFocusedSample
            }
            compat::UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::ConfirmBrowserDuplicateCleanup => {
                Self::ConfirmBrowserDuplicateCleanup
            }
            compat::UiAction::PlayRandomContentItem => Self::PlayRandomSample,
            compat::UiAction::PlayPreviousRandomContentItem => Self::PlayPreviousRandomSample,
            compat::UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta: delta }
            }
            compat::UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map: map },
            compat::UiAction::FocusBrowserPillEditorInput => Self::FocusBrowserTagSidebarInput,
            compat::UiAction::SetBrowserPillEditorInput { value } => {
                Self::SetBrowserTagSidebarInput { value: value }
            }
            compat::UiAction::CommitBrowserPillEditorInput => Self::CommitBrowserTagSidebarInput,
            compat::UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped: looped }
            }
            compat::UiAction::ToggleBrowserPillOption { label } => {
                Self::ToggleBrowserSidebarNormalTag { label: label }
            }
            compat::UiAction::FocusSpatialContentItem { content_id } => Self::FocusMapSample {
                sample_id: content_id,
            },
            compat::UiAction::SetPromptInput { value } => Self::SetPromptInput { value: value },
            compat::UiAction::StartBrowserRename => Self::StartBrowserRename,
            compat::UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            compat::UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            compat::UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::SetBrowserTriageMark { target } => Self::TagBrowserSelection {
                target: target.into(),
            },
            compat::UiAction::DeleteBrowserSelection => Self::DeleteBrowserSelection,
            compat::UiAction::NormalizeFocusedContentItem => Self::NormalizeFocusedBrowserSample,
            compat::UiAction::NormalizeWaveformSelectionOrLoadedContent => {
                Self::NormalizeWaveformSelectionOrSample
            }
            compat::UiAction::CropWaveformSelection => Self::CropWaveformSelection,
            compat::UiAction::CropWaveformSelectionToNewContentItem => {
                Self::CropWaveformSelectionToNewSample
            }
            compat::UiAction::TrimWaveformSelection => Self::TrimWaveformSelection,
            compat::UiAction::ReverseWaveformSelection => Self::ReverseWaveformSelection,
            compat::UiAction::FadeWaveformSelectionLeftToRight => {
                Self::FadeWaveformSelectionLeftToRight
            }
            compat::UiAction::FadeWaveformSelectionRightToLeft => {
                Self::FadeWaveformSelectionRightToLeft
            }
            compat::UiAction::MuteWaveformSelection => Self::MuteWaveformSelection,
            compat::UiAction::DeleteSelectedSliceMarkers => Self::DeleteSelectedSliceMarkers,
            compat::UiAction::ToggleWaveformSliceSelection { index } => {
                Self::ToggleWaveformSliceSelection { index: index }
            }
            compat::UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index: index }
            }
            compat::UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index: index }
            }
            compat::UiAction::MoveWaveformSliceFocus { delta } => {
                Self::MoveWaveformSliceFocus { delta: delta }
            }
            compat::UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            compat::UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            compat::UiAction::DeleteLoadedWaveformContent => Self::DeleteLoadedWaveformSample,
            compat::UiAction::SlideWaveformSelection { delta, fine } => {
                Self::SlideWaveformSelection {
                    delta: delta,
                    fine: fine,
                }
            }
            compat::UiAction::ConfirmPrompt => Self::ConfirmPrompt,
            compat::UiAction::CancelPrompt => Self::CancelPrompt,
            compat::UiAction::CancelProgress => Self::CancelProgress,
            compat::UiAction::CopySelectionToClipboard => Self::CopySelectionToClipboard,
            compat::UiAction::ToggleHotkeyOverlay => Self::ToggleHotkeyOverlay,
            compat::UiAction::CopyStatusLog => Self::CopyStatusLog,
            compat::UiAction::OpenFeedbackIssuePrompt => Self::OpenFeedbackIssuePrompt,
            compat::UiAction::MoveDiscardedItemsToFolder => Self::MoveTrashedSamplesToFolder,
            compat::UiAction::SetInputMonitoringEnabled { enabled } => {
                Self::SetInputMonitoringEnabled { enabled: enabled }
            }
            compat::UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled: enabled }
            }
            compat::UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled: enabled }
            }
            compat::UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled: enabled }
            }
            compat::UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            compat::UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            compat::UiAction::SetWaveformChannelView { stereo } => {
                Self::SetWaveformChannelView { stereo: stereo }
            }
            compat::UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled: enabled }
            }
            compat::UiAction::SetBpmSnapEnabled { enabled } => {
                Self::SetBpmSnapEnabled { enabled: enabled }
            }
            compat::UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled: enabled }
            }
            compat::UiAction::AdjustWaveformBpm { delta } => {
                Self::AdjustWaveformBpm { delta: delta }
            }
            compat::UiAction::SetWaveformBpmValue { value_tenths } => Self::SetWaveformBpmValue {
                value_tenths: value_tenths,
            },
            compat::UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled: enabled }
            }
            compat::UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled: enabled }
            }
            compat::UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            compat::UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            compat::UiAction::SetSliceModeEnabled { enabled } => {
                Self::SetSliceModeEnabled { enabled: enabled }
            }
            compat::UiAction::SetVolume { value_milli } => Self::SetVolume {
                value_milli: value_milli,
            },
            compat::UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            compat::UiAction::SeekWaveformPrecise { position_nanos } => Self::SeekWaveformPrecise {
                position_nanos: position_nanos,
            },
            compat::UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise {
                    position_nanos: position_nanos,
                }
            }
            compat::UiAction::SeekWaveform { position_milli } => Self::SeekWaveform {
                position_milli: position_milli,
            },
            compat::UiAction::SetWaveformCursor { position_milli } => Self::SetWaveformCursor {
                position_milli: position_milli,
            },
            compat::UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt {
                    anchor_micros: anchor_micros,
                }
            }
            compat::UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise {
                    anchor_nanos: anchor_nanos,
                }
            }
            compat::UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide {
                    anchor_micros: anchor_micros,
                }
            }
            compat::UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            compat::UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros: start_micros,
                end_micros: end_micros,
            },
            compat::UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            compat::UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve {
                    curve_milli: curve_milli,
                }
            }
            compat::UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve {
                    curve_milli: curve_milli,
                }
            }
            compat::UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            compat::UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
            },
            compat::UiAction::UpdateWaveformSelectionDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                over_browser_list,
                shift_down,
                alt_down,
            } => Self::UpdateWaveformSelectionDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                over_browser_list: over_browser_list,
                shift_down: shift_down,
                alt_down: alt_down,
            },
            compat::UiAction::FinishWaveformSelectionDrag => Self::FinishWaveformSelectionDrag,
            compat::UiAction::FinishWaveformSelectionRangeDrag => {
                Self::FinishWaveformSelectionRangeDrag
            }
            compat::UiAction::FinishWaveformSelectionSmartScaleDrag => {
                Self::FinishWaveformSelectionSmartScaleDrag
            }
            compat::UiAction::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformSelectionShift {
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            compat::UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            compat::UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            compat::UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            compat::UiAction::FinishWaveformEditSelectionDrag => {
                Self::FinishWaveformEditSelectionDrag
            }
            compat::UiAction::ClearWaveformSelection => Self::ClearWaveformSelection,
            compat::UiAction::ClearWaveformEditSelection => Self::ClearWaveformEditSelection,
            compat::UiAction::ClearWaveformSelections => Self::ClearWaveformSelections,
            compat::UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => Self::SetWaveformViewCenter {
                center_micros: center_micros,
                center_nanos: center_nanos,
            },
            compat::UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in: zoom_in,
                steps: steps,
                anchor_ratio_micros: anchor_ratio_micros,
            },
            compat::UiAction::ZoomWaveformToSelection => Self::ZoomWaveformToSelection,
            compat::UiAction::ZoomWaveformFull => Self::ZoomWaveformFull,
            compat::UiAction::Undo => Self::Undo,
            compat::UiAction::Redo => Self::Redo,
            compat::UiAction::CheckForUpdates => Self::CheckForUpdates,
            compat::UiAction::OpenUpdateLink => Self::OpenUpdateLink,
            compat::UiAction::InstallUpdate => Self::InstallUpdate,
            compat::UiAction::DismissUpdate => Self::DismissUpdate,
        }
    }
}

impl From<UiAction> for compat::UiAction {
    fn from(value: UiAction) -> Self {
        match value {
            UiAction::SelectColumn { index } => Self::SelectColumn { index: index },
            UiAction::MoveColumn { delta } => Self::MoveColumn { delta: delta },
            UiAction::ToggleTransport => Self::ToggleTransport,
            UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            UiAction::PlayFromStart => Self::PlayFromStart,
            UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            UiAction::PlayWaveformAtPrecise { position_nanos } => Self::PlayWaveformAtPrecise {
                position_nanos: position_nanos,
            },
            UiAction::HandleEscape => Self::HandleEscape,
            UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            UiAction::FocusFolderPanel { pane } => Self::FocusFolderPanel {
                pane: pane.map(Into::into),
            },
            UiAction::FocusLoadedSampleInBrowser => Self::FocusLoadedContentInList,
            UiAction::FocusBrowserSearch => Self::FocusBrowserSearch,
            UiAction::BlurBrowserSearch => Self::BlurBrowserSearch,
            UiAction::OpenAddSourceDialog => Self::OpenAddSourceDialog,
            UiAction::OpenOptionsMenu => Self::OpenOptionsMenu,
            UiAction::CloseOptionsPanel => Self::CloseOptionsPanel,
            UiAction::PickTrashFolder => Self::PickTrashFolder,
            UiAction::OpenTrashFolder => Self::OpenTrashFolder,
            UiAction::EditDefaultIdentifier => Self::EditDefaultIdentifier,
            UiAction::ShowOptionsOverview => Self::ShowOptionsOverview,
            UiAction::OpenAudioOutputHostPicker => Self::OpenPrimaryGroupPicker,
            UiAction::OpenAudioOutputDevicePicker => Self::OpenPrimaryItemPicker,
            UiAction::OpenAudioOutputSampleRatePicker => Self::OpenPrimaryNumberPicker,
            UiAction::OpenAudioInputHostPicker => Self::OpenSecondaryGroupPicker,
            UiAction::OpenAudioInputDevicePicker => Self::OpenSecondaryItemPicker,
            UiAction::OpenAudioInputSampleRatePicker => Self::OpenSecondaryNumberPicker,
            UiAction::SetAudioOutputHost { host_id } => Self::SetPrimaryGroup { group_id: host_id },
            UiAction::SetAudioOutputDevice { device_name } => Self::SetPrimaryItem {
                item_name: device_name,
            },
            UiAction::SetAudioOutputSampleRate { sample_rate } => {
                Self::SetPrimaryNumber { value: sample_rate }
            }
            UiAction::SetAudioInputHost { host_id } => {
                Self::SetSecondaryGroup { group_id: host_id }
            }
            UiAction::SetAudioInputDevice { device_name } => Self::SetSecondaryItem {
                item_name: device_name,
            },
            UiAction::SetAudioInputSampleRate { sample_rate } => {
                Self::SetSecondaryNumber { value: sample_rate }
            }
            UiAction::FocusFolderSearch { pane } => Self::FocusFolderSearch {
                pane: pane.map(Into::into),
            },
            UiAction::SetFolderSearch { pane, query } => Self::SetFolderSearch {
                pane: pane.map(Into::into),
                query: query,
            },
            UiAction::ToggleShowAllFolders { pane } => Self::ToggleShowAllFolders {
                pane: pane.map(Into::into),
            },
            UiAction::ToggleFolderFlattenedView { pane } => Self::ToggleFolderFlattenedView {
                pane: pane.map(Into::into),
            },
            UiAction::FocusSourceRow { pane, index } => Self::FocusSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::SelectSourceRow { pane, index } => Self::SelectSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta: delta },
            UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            UiAction::ReloadSourceRow { pane, index } => Self::ReloadSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::HardSyncSourceRow { pane, index } => Self::HardSyncSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::OpenSourceFolderRow { pane, index } => Self::OpenSourceFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::RemoveSourceRow { pane, index } => Self::RemoveSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::FocusFolderRow { pane, index } => Self::FocusFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::ActivateFolderRow { pane, index } => Self::ActivateFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::ToggleFolderRowExpanded { pane, index } => Self::ToggleFolderRowExpanded {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta: delta },
            UiAction::StartNewFolder => Self::StartNewFolder,
            UiAction::StartNewFolderAtFolderRow { pane, index } => {
                Self::StartNewFolderAtFolderRow {
                    pane: pane.map(Into::into),
                    index: index,
                }
            }
            UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            UiAction::SetFolderCreateInput { value } => Self::SetFolderCreateInput { value: value },
            UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            UiAction::StartFolderRename => Self::StartFolderRename,
            UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta: delta },
            UiAction::SetBrowserViewStart { visible_row } => Self::SetBrowserViewStart {
                visible_row: visible_row,
            },
            UiAction::FocusBrowserRow { visible_row } => Self::FocusBrowserRow {
                visible_row: visible_row,
            },
            UiAction::SetCompareAnchorFromFocusedBrowserSample => {
                Self::SetCompareAnchorFromFocusedContent
            }
            UiAction::CommitFocusedBrowserRow => Self::CommitFocusedBrowserRow,
            UiAction::SaveWaveformSelectionToBrowser => Self::SaveWaveformSelectionToBrowser,
            UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
                Self::SaveWaveformSelectionToBrowserWithKeep2
            }
            UiAction::CommitWaveformEditFades => Self::CommitWaveformEditFades,
            UiAction::DetectWaveformSilenceSlices => Self::DetectWaveformSilenceSlices,
            UiAction::DetectWaveformExactDuplicateSlices => {
                Self::DetectWaveformExactDuplicateSlices
            }
            UiAction::CleanWaveformExactDuplicateSlices => Self::CleanWaveformExactDuplicateSlices,
            UiAction::ToggleBrowserRowSelection { visible_row } => {
                Self::ToggleBrowserRowSelection {
                    visible_row: visible_row,
                }
            }
            UiAction::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartContentItemDrag {
                visible_row: visible_row,
                pointer_x: pointer_x,
                pointer_y: pointer_y,
            },
            UiAction::UpdateBrowserSampleDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            } => Self::UpdateContentItemDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                shift_down: shift_down,
                alt_down: alt_down,
            },
            UiAction::FinishBrowserSampleDrag => Self::FinishContentItemDrag,
            UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow {
                    visible_row: visible_row,
                }
            }
            UiAction::AddRangeBrowserSelection { visible_row } => Self::AddRangeBrowserSelection {
                visible_row: visible_row,
            },
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta: delta }
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta: delta }
            }
            UiAction::ToggleFocusedBrowserRowSelection => Self::ToggleFocusedBrowserRowSelection,
            UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query: query },
            UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter {
                    level: level,
                    invert: invert,
                }
            }
            UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter {
                    bucket: bucket.into(),
                    invert: invert,
                }
            }
            UiAction::ToggleBrowserSampleMark => Self::ToggleContentMark,
            UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            UiAction::ToggleBrowserTagNamedFilter { invert } => {
                Self::ToggleBrowserDerivedLabelFilter { invert: invert }
            }
            UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            UiAction::ToggleBrowserTagSidebar => Self::ToggleBrowserPillEditor,
            UiAction::ToggleBrowserTagSidebarAutoRename => {
                Self::ToggleBrowserPillEditorPrimaryAction
            }
            UiAction::ToggleBrowserDuplicateCleanupMode => Self::ToggleBrowserDuplicateCleanupMode,
            UiAction::FocusPreviousBrowserHistory => Self::FocusPreviousBrowserHistory,
            UiAction::FocusNextBrowserHistory => Self::FocusNextBrowserHistory,
            UiAction::ToggleFindSimilarFocusedSample => Self::ToggleFindSimilarFocusedContent,
            UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep {
                    visible_row: visible_row,
                }
            }
            UiAction::ConfirmBrowserDuplicateCleanup => Self::ConfirmBrowserDuplicateCleanup,
            UiAction::PlayRandomSample => Self::PlayRandomContentItem,
            UiAction::PlayPreviousRandomSample => Self::PlayPreviousRandomContentItem,
            UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta: delta }
            }
            UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map: map },
            UiAction::FocusBrowserTagSidebarInput => Self::FocusBrowserPillEditorInput,
            UiAction::SetBrowserTagSidebarInput { value } => {
                Self::SetBrowserPillEditorInput { value: value }
            }
            UiAction::CommitBrowserTagSidebarInput => Self::CommitBrowserPillEditorInput,
            UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped: looped }
            }
            UiAction::ToggleBrowserSidebarNormalTag { label } => {
                Self::ToggleBrowserPillOption { label: label }
            }
            UiAction::FocusMapSample { sample_id } => Self::FocusSpatialContentItem {
                content_id: sample_id,
            },
            UiAction::SetPromptInput { value } => Self::SetPromptInput { value: value },
            UiAction::StartBrowserRename => Self::StartBrowserRename,
            UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection {
                    visible_row: visible_row,
                }
            }
            UiAction::TagBrowserSelection { target } => Self::SetBrowserTriageMark {
                target: target.into(),
            },
            UiAction::DeleteBrowserSelection => Self::DeleteBrowserSelection,
            UiAction::NormalizeFocusedBrowserSample => Self::NormalizeFocusedContentItem,
            UiAction::NormalizeWaveformSelectionOrSample => {
                Self::NormalizeWaveformSelectionOrLoadedContent
            }
            UiAction::CropWaveformSelection => Self::CropWaveformSelection,
            UiAction::CropWaveformSelectionToNewSample => {
                Self::CropWaveformSelectionToNewContentItem
            }
            UiAction::TrimWaveformSelection => Self::TrimWaveformSelection,
            UiAction::ReverseWaveformSelection => Self::ReverseWaveformSelection,
            UiAction::FadeWaveformSelectionLeftToRight => Self::FadeWaveformSelectionLeftToRight,
            UiAction::FadeWaveformSelectionRightToLeft => Self::FadeWaveformSelectionRightToLeft,
            UiAction::MuteWaveformSelection => Self::MuteWaveformSelection,
            UiAction::DeleteSelectedSliceMarkers => Self::DeleteSelectedSliceMarkers,
            UiAction::ToggleWaveformSliceSelection { index } => {
                Self::ToggleWaveformSliceSelection { index: index }
            }
            UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index: index }
            }
            UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index: index }
            }
            UiAction::MoveWaveformSliceFocus { delta } => {
                Self::MoveWaveformSliceFocus { delta: delta }
            }
            UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            UiAction::DeleteLoadedWaveformSample => Self::DeleteLoadedWaveformContent,
            UiAction::SlideWaveformSelection { delta, fine } => Self::SlideWaveformSelection {
                delta: delta,
                fine: fine,
            },
            UiAction::ConfirmPrompt => Self::ConfirmPrompt,
            UiAction::CancelPrompt => Self::CancelPrompt,
            UiAction::CancelProgress => Self::CancelProgress,
            UiAction::CopySelectionToClipboard => Self::CopySelectionToClipboard,
            UiAction::ToggleHotkeyOverlay => Self::ToggleHotkeyOverlay,
            UiAction::CopyStatusLog => Self::CopyStatusLog,
            UiAction::OpenFeedbackIssuePrompt => Self::OpenFeedbackIssuePrompt,
            UiAction::MoveTrashedSamplesToFolder => Self::MoveDiscardedItemsToFolder,
            UiAction::SetInputMonitoringEnabled { enabled } => {
                Self::SetInputMonitoringEnabled { enabled: enabled }
            }
            UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled: enabled }
            }
            UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled: enabled }
            }
            UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled: enabled }
            }
            UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            UiAction::SetWaveformChannelView { stereo } => {
                Self::SetWaveformChannelView { stereo: stereo }
            }
            UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled: enabled }
            }
            UiAction::SetBpmSnapEnabled { enabled } => Self::SetBpmSnapEnabled { enabled: enabled },
            UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled: enabled }
            }
            UiAction::AdjustWaveformBpm { delta } => Self::AdjustWaveformBpm { delta: delta },
            UiAction::SetWaveformBpmValue { value_tenths } => Self::SetWaveformBpmValue {
                value_tenths: value_tenths,
            },
            UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled: enabled }
            }
            UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled: enabled }
            }
            UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            UiAction::SetSliceModeEnabled { enabled } => {
                Self::SetSliceModeEnabled { enabled: enabled }
            }
            UiAction::SetVolume { value_milli } => Self::SetVolume {
                value_milli: value_milli,
            },
            UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            UiAction::SeekWaveformPrecise { position_nanos } => Self::SeekWaveformPrecise {
                position_nanos: position_nanos,
            },
            UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise {
                    position_nanos: position_nanos,
                }
            }
            UiAction::SeekWaveform { position_milli } => Self::SeekWaveform {
                position_milli: position_milli,
            },
            UiAction::SetWaveformCursor { position_milli } => Self::SetWaveformCursor {
                position_milli: position_milli,
            },
            UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt {
                    anchor_micros: anchor_micros,
                }
            }
            UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise {
                    anchor_nanos: anchor_nanos,
                }
            }
            UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide {
                    anchor_micros: anchor_micros,
                }
            }
            UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide {
                    position_micros: position_micros,
                }
            }
            UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros: start_micros,
                end_micros: end_micros,
            },
            UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve {
                    curve_milli: curve_milli,
                }
            }
            UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve {
                    curve_milli: curve_milli,
                }
            }
            UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
            },
            UiAction::UpdateWaveformSelectionDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                over_browser_list,
                shift_down,
                alt_down,
            } => Self::UpdateWaveformSelectionDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                over_browser_list: over_browser_list,
                shift_down: shift_down,
                alt_down: alt_down,
            },
            UiAction::FinishWaveformSelectionDrag => Self::FinishWaveformSelectionDrag,
            UiAction::FinishWaveformSelectionRangeDrag => Self::FinishWaveformSelectionRangeDrag,
            UiAction::FinishWaveformSelectionSmartScaleDrag => {
                Self::FinishWaveformSelectionSmartScaleDrag
            }
            UiAction::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformSelectionShift {
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            UiAction::FinishWaveformEditSelectionDrag => Self::FinishWaveformEditSelectionDrag,
            UiAction::ClearWaveformSelection => Self::ClearWaveformSelection,
            UiAction::ClearWaveformEditSelection => Self::ClearWaveformEditSelection,
            UiAction::ClearWaveformSelections => Self::ClearWaveformSelections,
            UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => Self::SetWaveformViewCenter {
                center_micros: center_micros,
                center_nanos: center_nanos,
            },
            UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in: zoom_in,
                steps: steps,
                anchor_ratio_micros: anchor_ratio_micros,
            },
            UiAction::ZoomWaveformToSelection => Self::ZoomWaveformToSelection,
            UiAction::ZoomWaveformFull => Self::ZoomWaveformFull,
            UiAction::Undo => Self::Undo,
            UiAction::Redo => Self::Redo,
            UiAction::CheckForUpdates => Self::CheckForUpdates,
            UiAction::OpenUpdateLink => Self::OpenUpdateLink,
            UiAction::InstallUpdate => Self::InstallUpdate,
            UiAction::DismissUpdate => Self::DismissUpdate,
        }
    }
}

pub(super) fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    radiant::compat::legacy_shell::run_native_vello_app(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    )
}

pub(super) fn run_native_vello_app_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    let report = radiant::compat::legacy_shell::run_native_vello_app_with_artifacts(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    );
    native_run_report_from_radiant(report)
}

pub(super) fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    let compat_model = radiant::compat::legacy_shell::AppModel::from(model);
    radiant::compat::legacy_shell::capture_gui_automation_snapshot(viewport, &compat_model).into()
}

#[cfg(test)]
pub(super) fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> impl serde::Serialize {
    let compat_model = radiant::compat::legacy_shell::AppModel::from(model);
    radiant::compat::legacy_shell::capture_native_shell_shot_snapshot(name, viewport, &compat_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_run_options_map_field_for_field_to_radiant_compat_options() {
        let options = NativeRunOptions {
            title: String::from("Sempal test host"),
            inner_size: Some([1280.0, 720.0]),
            min_inner_size: Some([640.0, 360.0]),
            maximized: true,
            decorations: false,
            icon: Some(WindowIconRgba {
                rgba: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
            }),
            target_fps: 90,
        };

        let compat: radiant::compat::legacy_shell::NativeRunOptions = options.into();

        assert_eq!(compat.title, "Sempal test host");
        assert_eq!(compat.inner_size, Some([1280.0, 720.0]));
        assert_eq!(compat.min_inner_size, Some([640.0, 360.0]));
        assert!(compat.maximized);
        assert!(!compat.decorations);
        assert_eq!(compat.target_fps, 90);
        let icon = compat.icon.expect("icon should be forwarded");
        assert_eq!(icon.rgba, vec![255, 0, 0, 255]);
        assert_eq!(icon.width, 1);
        assert_eq!(icon.height, 1);
    }
}
