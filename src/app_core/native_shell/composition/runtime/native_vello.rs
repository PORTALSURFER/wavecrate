//! Sempal native-shell adapter for the temporary Radiant legacy runtime path.

use super::{NativeRunOptions, NativeRunReport, NativeRuntimeArtifacts, WindowIconRgba};
use crate::app::{
    controller::ui::hotkeys::{self, KeyPress},
    state::FocusContext,
};
use crate::app_core::actions::{
    NativeAppBridge, NativeAppModel, NativeBrowserTagTarget as BrowserTagTarget,
    NativeFrameBuildResult, NativeGuiAutomationSnapshot, NativeMotionModel, NativeUiAction,
    NativeUiAction as UiAction, native_shell_dtos::*,
};
use crate::gui::automation as gui_automation;
use crate::gui::{
    native_shell::{NativeShellState, ShellLayout, ShellLayoutRuntime, StyleTokens},
    types::Vector2,
};
use radiant::compat::legacy_shell as compat;
use radiant::gui::{
    focus::FocusSurface as RadiantFocusSurface, frame::FrameBuildResult as RadiantFrameBuildResult,
    input::KeyPress as RadiantKeyPress, shortcuts::ShortcutResolution as RadiantShortcutResolution,
};
use std::{collections::BTreeMap, sync::Arc};

/// Converts app-level Vello launch options into the generic `radiant` runtime representation.
///
/// Mapping is intentionally field-for-field to preserve behavior and avoid
/// hidden launch-time mutations.
impl From<NativeRunOptions> for radiant::gui_runtime::NativeRunOptions {
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
impl From<WindowIconRgba> for radiant::gui_runtime::WindowIconRgba {
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
        pending_chord: Option<RadiantKeyPress>,
        press: RadiantKeyPress,
        focus: RadiantFocusSurface,
    ) -> RadiantShortcutResolution<radiant::compat::legacy_shell::UiAction> {
        let resolution = hotkeys::resolve_hotkey_press(
            pending_chord.map(keypress_from_radiant),
            keypress_from_radiant(press),
            focus_context_from_radiant(focus),
        );
        RadiantShortcutResolution {
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

    fn observe_frame_result(&mut self, result: RadiantFrameBuildResult) {
        self.inner
            .observe_frame_result(NativeFrameBuildResult::from(result));
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.inner
            .on_runtime_exit()
            .and_then(|artifact| serde_json::to_value(artifact).ok())
    }
}

fn focus_context_from_radiant(focus: RadiantFocusSurface) -> FocusContext {
    match focus {
        RadiantFocusSurface::None => FocusContext::None,
        RadiantFocusSurface::Timeline => FocusContext::Waveform,
        RadiantFocusSurface::ContentList => FocusContext::SampleBrowser,
        RadiantFocusSurface::NavigationTree => FocusContext::SourceFolders,
        RadiantFocusSurface::NavigationList => FocusContext::SourcesList,
    }
}

fn keypress_from_radiant(press: RadiantKeyPress) -> KeyPress {
    KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

fn keypress_to_radiant(press: KeyPress) -> RadiantKeyPress {
    RadiantKeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

impl From<compat::ContentTriageTarget> for BrowserTagTarget {
    fn from(value: compat::ContentTriageTarget) -> Self {
        match value {
            compat::ContentTriageTarget::Negative => Self::Trash,
            compat::ContentTriageTarget::Neutral => Self::Neutral,
            compat::ContentTriageTarget::Positive => Self::Keep,
        }
    }
}

impl From<BrowserTagTarget> for compat::ContentTriageTarget {
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
            compat::UiAction::FocusContentPanel => Self::FocusBrowserPanel,
            compat::UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            compat::UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            compat::UiAction::FocusFolderPanel { pane } => Self::FocusFolderPanel {
                pane: pane.map(Into::into),
            },
            compat::UiAction::FocusLoadedContentInList => Self::FocusLoadedSampleInBrowser,
            compat::UiAction::FocusContentSearch => Self::FocusBrowserSearch,
            compat::UiAction::BlurContentSearch => Self::BlurBrowserSearch,
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
            compat::UiAction::MoveContentFocus { delta } => Self::MoveBrowserFocus { delta: delta },
            compat::UiAction::SetContentViewStart { visible_row } => Self::SetBrowserViewStart {
                visible_row: visible_row,
            },
            compat::UiAction::FocusContentRow { visible_row } => Self::FocusBrowserRow {
                visible_row: visible_row,
            },
            compat::UiAction::SetCompareAnchorFromFocusedContent => {
                Self::SetCompareAnchorFromFocusedBrowserSample
            }
            compat::UiAction::CommitFocusedContentRow => Self::CommitFocusedBrowserRow,
            compat::UiAction::SaveWaveformSelectionAsContent => {
                Self::SaveWaveformSelectionToBrowser
            }
            compat::UiAction::SaveWaveformSelectionAsAlternateContent => {
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
            compat::UiAction::ToggleContentRowSelection { visible_row } => {
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
            compat::UiAction::ExtendContentSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::AddRangeContentSelectionToRow { visible_row } => {
                Self::AddRangeBrowserSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::ExtendContentSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta: delta }
            }
            compat::UiAction::AddRangeContentSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta: delta }
            }
            compat::UiAction::ToggleFocusedContentRowSelection => {
                Self::ToggleFocusedBrowserRowSelection
            }
            compat::UiAction::SelectAllContentRows => Self::SelectAllBrowserRows,
            compat::UiAction::SetContentSearch { query } => Self::SetBrowserSearch { query: query },
            compat::UiAction::ToggleContentRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter {
                    level: level,
                    invert: invert,
                }
            }
            compat::UiAction::ToggleContentRecencyFilter { chip, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter {
                    bucket: chip.into(),
                    invert: invert,
                }
            }
            compat::UiAction::ToggleContentMark => Self::ToggleBrowserSampleMark,
            compat::UiAction::ToggleContentMarkedFilter => Self::ToggleBrowserMarkedFilter,
            compat::UiAction::ToggleContentDerivedLabelFilter { invert } => {
                Self::ToggleBrowserTagNamedFilter { invert: invert }
            }
            compat::UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            compat::UiAction::ToggleContentPillEditor => Self::ToggleBrowserTagSidebar,
            compat::UiAction::ToggleContentPillEditorPrimaryAction => {
                Self::ToggleBrowserTagSidebarAutoRename
            }
            compat::UiAction::ToggleContentDuplicateCleanupMode => {
                Self::ToggleBrowserDuplicateCleanupMode
            }
            compat::UiAction::FocusPreviousContentHistory => Self::FocusPreviousBrowserHistory,
            compat::UiAction::FocusNextContentHistory => Self::FocusNextBrowserHistory,
            compat::UiAction::ToggleFindSimilarFocusedContent => {
                Self::ToggleFindSimilarFocusedSample
            }
            compat::UiAction::ToggleContentDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::ConfirmContentDuplicateCleanup => {
                Self::ConfirmBrowserDuplicateCleanup
            }
            compat::UiAction::PlayRandomContentItem => Self::PlayRandomSample,
            compat::UiAction::PlayPreviousRandomContentItem => Self::PlayPreviousRandomSample,
            compat::UiAction::AdjustSelectedContentRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta: delta }
            }
            compat::UiAction::SetContentTab { map } => Self::SetBrowserTab { map: map },
            compat::UiAction::FocusContentPillEditorInput => Self::FocusBrowserTagSidebarInput,
            compat::UiAction::SetContentPillEditorInput { value } => {
                Self::SetBrowserTagSidebarInput { value: value }
            }
            compat::UiAction::CommitContentPillEditorInput => Self::CommitBrowserTagSidebarInput,
            compat::UiAction::SetContentPlaybackLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped: looped }
            }
            compat::UiAction::ToggleContentPillOption { label } => {
                Self::ToggleBrowserSidebarNormalTag { label: label }
            }
            compat::UiAction::FocusSpatialContentItem { content_id } => Self::FocusMapSample {
                sample_id: content_id,
            },
            compat::UiAction::SetPromptInput { value } => Self::SetPromptInput { value: value },
            compat::UiAction::StartContentRename => Self::StartBrowserRename,
            compat::UiAction::ConfirmContentRename => Self::ConfirmBrowserRename,
            compat::UiAction::CancelContentRename => Self::CancelBrowserRename,
            compat::UiAction::AutoRenameContentSelection { visible_row } => {
                Self::AutoRenameBrowserSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::SetContentTriageMark { target } => Self::TagBrowserSelection {
                target: target.into(),
            },
            compat::UiAction::DeleteContentSelection => Self::DeleteBrowserSelection,
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
                over_content_list,
                shift_down,
                alt_down,
            } => Self::UpdateWaveformSelectionDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                over_browser_list: over_content_list,
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
            UiAction::FocusBrowserPanel => Self::FocusContentPanel,
            UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            UiAction::FocusFolderPanel { pane } => Self::FocusFolderPanel {
                pane: pane.map(Into::into),
            },
            UiAction::FocusLoadedSampleInBrowser => Self::FocusLoadedContentInList,
            UiAction::FocusBrowserSearch => Self::FocusContentSearch,
            UiAction::BlurBrowserSearch => Self::BlurContentSearch,
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
            UiAction::MoveBrowserFocus { delta } => Self::MoveContentFocus { delta: delta },
            UiAction::SetBrowserViewStart { visible_row } => Self::SetContentViewStart {
                visible_row: visible_row,
            },
            UiAction::FocusBrowserRow { visible_row } => Self::FocusContentRow {
                visible_row: visible_row,
            },
            UiAction::SetCompareAnchorFromFocusedBrowserSample => {
                Self::SetCompareAnchorFromFocusedContent
            }
            UiAction::CommitFocusedBrowserRow => Self::CommitFocusedContentRow,
            UiAction::SaveWaveformSelectionToBrowser => Self::SaveWaveformSelectionAsContent,
            UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
                Self::SaveWaveformSelectionAsAlternateContent
            }
            UiAction::CommitWaveformEditFades => Self::CommitWaveformEditFades,
            UiAction::DetectWaveformSilenceSlices => Self::DetectWaveformSilenceSlices,
            UiAction::DetectWaveformExactDuplicateSlices => {
                Self::DetectWaveformExactDuplicateSlices
            }
            UiAction::CleanWaveformExactDuplicateSlices => Self::CleanWaveformExactDuplicateSlices,
            UiAction::ToggleBrowserRowSelection { visible_row } => {
                Self::ToggleContentRowSelection {
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
                Self::ExtendContentSelectionToRow {
                    visible_row: visible_row,
                }
            }
            UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeContentSelectionToRow {
                    visible_row: visible_row,
                }
            }
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendContentSelectionFromFocus { delta: delta }
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeContentSelectionFromFocus { delta: delta }
            }
            UiAction::ToggleFocusedBrowserRowSelection => Self::ToggleFocusedContentRowSelection,
            UiAction::SelectAllBrowserRows => Self::SelectAllContentRows,
            UiAction::SetBrowserSearch { query } => Self::SetContentSearch { query: query },
            UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleContentRatingFilter {
                    level: level,
                    invert: invert,
                }
            }
            UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleContentRecencyFilter {
                    chip: bucket.into(),
                    invert: invert,
                }
            }
            UiAction::ToggleBrowserSampleMark => Self::ToggleContentMark,
            UiAction::ToggleBrowserMarkedFilter => Self::ToggleContentMarkedFilter,
            UiAction::ToggleBrowserTagNamedFilter { invert } => {
                Self::ToggleContentDerivedLabelFilter { invert: invert }
            }
            UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            UiAction::ToggleBrowserTagSidebar => Self::ToggleContentPillEditor,
            UiAction::ToggleBrowserTagSidebarAutoRename => {
                Self::ToggleContentPillEditorPrimaryAction
            }
            UiAction::ToggleBrowserDuplicateCleanupMode => Self::ToggleContentDuplicateCleanupMode,
            UiAction::FocusPreviousBrowserHistory => Self::FocusPreviousContentHistory,
            UiAction::FocusNextBrowserHistory => Self::FocusNextContentHistory,
            UiAction::ToggleFindSimilarFocusedSample => Self::ToggleFindSimilarFocusedContent,
            UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleContentDuplicateCleanupKeep {
                    visible_row: visible_row,
                }
            }
            UiAction::ConfirmBrowserDuplicateCleanup => Self::ConfirmContentDuplicateCleanup,
            UiAction::PlayRandomSample => Self::PlayRandomContentItem,
            UiAction::PlayPreviousRandomSample => Self::PlayPreviousRandomContentItem,
            UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedContentRating { delta: delta }
            }
            UiAction::SetBrowserTab { map } => Self::SetContentTab { map: map },
            UiAction::FocusBrowserTagSidebarInput => Self::FocusContentPillEditorInput,
            UiAction::SetBrowserTagSidebarInput { value } => {
                Self::SetContentPillEditorInput { value: value }
            }
            UiAction::CommitBrowserTagSidebarInput => Self::CommitContentPillEditorInput,
            UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetContentPlaybackLooped { looped: looped }
            }
            UiAction::ToggleBrowserSidebarNormalTag { label } => {
                Self::ToggleContentPillOption { label: label }
            }
            UiAction::FocusMapSample { sample_id } => Self::FocusSpatialContentItem {
                content_id: sample_id,
            },
            UiAction::SetPromptInput { value } => Self::SetPromptInput { value: value },
            UiAction::StartBrowserRename => Self::StartContentRename,
            UiAction::ConfirmBrowserRename => Self::ConfirmContentRename,
            UiAction::CancelBrowserRename => Self::CancelContentRename,
            UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameContentSelection {
                    visible_row: visible_row,
                }
            }
            UiAction::TagBrowserSelection { target } => Self::SetContentTriageMark {
                target: target.into(),
            },
            UiAction::DeleteBrowserSelection => Self::DeleteContentSelection,
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
                over_content_list: over_browser_list,
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

fn retained_vec_from_compat<T, U>(value: compat::RetainedVec<T>) -> RetainedVec<U>
where
    T: Clone + Into<U>,
{
    value
        .as_slice()
        .iter()
        .cloned()
        .map(Into::into)
        .collect::<Vec<_>>()
        .into()
}

fn retained_vec_to_compat<T, U>(value: RetainedVec<T>) -> compat::RetainedVec<U>
where
    T: Clone + Into<U>,
{
    value
        .as_slice()
        .iter()
        .cloned()
        .map(Into::into)
        .collect::<Vec<_>>()
        .into()
}

impl From<compat::FocusContextModel> for FocusContextModel {
    fn from(value: compat::FocusContextModel) -> Self {
        match value {
            compat::FocusContextModel::None => Self::None,
            compat::FocusContextModel::Timeline => Self::Waveform,
            compat::FocusContextModel::ContentList => Self::SampleBrowser,
            compat::FocusContextModel::NavigationTree => Self::SourceFolders,
            compat::FocusContextModel::NavigationList => Self::SourcesList,
        }
    }
}

impl From<FocusContextModel> for compat::FocusContextModel {
    fn from(value: FocusContextModel) -> Self {
        match value {
            FocusContextModel::None => Self::None,
            FocusContextModel::Waveform => Self::Timeline,
            FocusContextModel::SampleBrowser => Self::ContentList,
            FocusContextModel::SourceFolders => Self::NavigationTree,
            FocusContextModel::SourcesList => Self::NavigationList,
        }
    }
}

impl From<compat::SourcesPanelModel> for SourcesPanelModel {
    fn from(value: compat::SourcesPanelModel) -> Self {
        Self {
            header: value.header,
            search_query: value.search_query,
            active_folder_pane: value.active_folder_pane.into(),
            upper_folder_pane: value.upper_folder_pane.into(),
            lower_folder_pane: value.lower_folder_pane.into(),
            tree_search_query: value.tree_search_query,
            show_all_items: value.show_all_items,
            can_toggle_show_all_items: value.can_toggle_show_all_items,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            selected_row: value.selected_row,
            loading_row: value.loading_row,
            mutation_busy_row: value.mutation_busy_row,
            focused_tree_row: value.focused_tree_row,
            rows: retained_vec_from_compat(value.rows),
            tree_rows: retained_vec_from_compat(value.tree_rows),
            tree_actions: value.tree_actions.into(),
            recovery: value.recovery.into(),
        }
    }
}

impl From<SourcesPanelModel> for compat::SourcesPanelModel {
    fn from(value: SourcesPanelModel) -> Self {
        Self {
            header: value.header,
            search_query: value.search_query,
            active_folder_pane: value.active_folder_pane.into(),
            upper_folder_pane: value.upper_folder_pane.into(),
            lower_folder_pane: value.lower_folder_pane.into(),
            tree_search_query: value.tree_search_query,
            show_all_items: value.show_all_items,
            can_toggle_show_all_items: value.can_toggle_show_all_items,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            selected_row: value.selected_row,
            loading_row: value.loading_row,
            mutation_busy_row: value.mutation_busy_row,
            focused_tree_row: value.focused_tree_row,
            rows: retained_vec_to_compat(value.rows),
            tree_rows: retained_vec_to_compat(value.tree_rows),
            tree_actions: value.tree_actions.into(),
            recovery: value.recovery.into(),
        }
    }
}

impl From<&SourcesPanelModel> for compat::SourcesPanelModel {
    fn from(value: &SourcesPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserPanelModel> for BrowserPanelModel {
    fn from(value: compat::BrowserPanelModel) -> Self {
        Self {
            visible_count: value.visible_count,
            selected_visible_row: value.selected_visible_row,
            autoscroll: value.autoscroll,
            view_start_row: value.view_start_row,
            selected_path_count: value.selected_item_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_recency_filters,
            marked_filter_active: value.marked_filter_active,
            tag_named_filter_active: value.derived_label_filter_active,
            tag_named_filter_negated: value.derived_label_filter_negated,
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            source_loading: value.data_loading,
            metadata_pending: value.metadata_pending,
            file_op_pending: value.mutation_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_sample_label: value.focused_item_label,
            tag_sidebar: value.pill_editor.into(),
            anchor_visible_row: value.anchor_visible_row,
            rows: retained_vec_from_compat(value.rows),
        }
    }
}

impl From<BrowserPanelModel> for compat::BrowserPanelModel {
    fn from(value: BrowserPanelModel) -> Self {
        Self {
            visible_count: value.visible_count,
            selected_visible_row: value.selected_visible_row,
            autoscroll: value.autoscroll,
            view_start_row: value.view_start_row,
            selected_item_count: value.selected_path_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_recency_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            derived_label_filter_active: value.tag_named_filter_active,
            derived_label_filter_negated: value.tag_named_filter_negated,
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            data_loading: value.source_loading,
            metadata_pending: value.metadata_pending,
            mutation_pending: value.file_op_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_item_label: value.focused_sample_label,
            pill_editor: value.tag_sidebar.into(),
            anchor_visible_row: value.anchor_visible_row,
            rows: retained_vec_to_compat(value.rows),
        }
    }
}

impl From<&BrowserPanelModel> for compat::BrowserPanelModel {
    fn from(value: &BrowserPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserChromeModel> for BrowserChromeModel {
    fn from(value: compat::BrowserChromeModel) -> Self {
        Self {
            samples_tab_label: value.items_tab_label,
            sample_column_label: value.item_column_label,
            map_tab_label: value.map_tab_label,
            tag_editor_label: value.pill_editor_label,
            search_prefix_label: value.search_prefix_label,
            search_placeholder: value.search_placeholder,
            activity_ready_label: value.activity_ready_label,
            activity_busy_label: value.activity_busy_label,
            sort_prefix_label: value.sort_prefix_label,
            sort_order_label: value.sort_order_label,
            similarity_toggle_label: value.similarity_toggle_label,
            item_count_label: value.item_count_label,
        }
    }
}

impl From<BrowserChromeModel> for compat::BrowserChromeModel {
    fn from(value: BrowserChromeModel) -> Self {
        Self {
            items_tab_label: value.samples_tab_label,
            item_column_label: value.sample_column_label,
            map_tab_label: value.map_tab_label,
            pill_editor_label: value.tag_editor_label,
            search_prefix_label: value.search_prefix_label,
            search_placeholder: value.search_placeholder,
            activity_ready_label: value.activity_ready_label,
            activity_busy_label: value.activity_busy_label,
            sort_prefix_label: value.sort_prefix_label,
            sort_order_label: value.sort_order_label,
            similarity_toggle_label: value.similarity_toggle_label,
            item_count_label: value.item_count_label,
        }
    }
}

impl From<&BrowserChromeModel> for compat::BrowserChromeModel {
    fn from(value: &BrowserChromeModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserActionsModel> for BrowserActionsModel {
    fn from(value: compat::BrowserActionsModel) -> Self {
        Self {
            can_rename: value.can_rename,
            can_delete: value.can_delete,
            can_tag: value.can_edit_pills,
            can_normalize_focused_sample: value.can_process_focused_item,
            can_loop_crossfade_focused_sample: value.can_open_focused_item_flow,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            tag_sidebar_open: value.pill_editor_open,
        }
    }
}

impl From<BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: BrowserActionsModel) -> Self {
        Self {
            can_rename: value.can_rename,
            can_delete: value.can_delete,
            can_edit_pills: value.can_tag,
            can_process_focused_item: value.can_normalize_focused_sample,
            can_open_focused_item_flow: value.can_loop_crossfade_focused_sample,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            pill_editor_open: value.tag_sidebar_open,
        }
    }
}

impl From<&BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: &BrowserActionsModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::PairedPickerTargetModel> for AudioPickerTargetModel {
    fn from(value: compat::PairedPickerTargetModel) -> Self {
        match value {
            compat::PairedPickerTargetModel::PrimaryGroup => Self::OutputHost,
            compat::PairedPickerTargetModel::PrimaryItem => Self::OutputDevice,
            compat::PairedPickerTargetModel::PrimaryNumber => Self::OutputSampleRate,
            compat::PairedPickerTargetModel::SecondaryGroup => Self::InputHost,
            compat::PairedPickerTargetModel::SecondaryItem => Self::InputDevice,
            compat::PairedPickerTargetModel::SecondaryNumber => Self::InputSampleRate,
        }
    }
}

impl From<AudioPickerTargetModel> for compat::PairedPickerTargetModel {
    fn from(value: AudioPickerTargetModel) -> Self {
        match value {
            AudioPickerTargetModel::OutputHost => Self::PrimaryGroup,
            AudioPickerTargetModel::OutputDevice => Self::PrimaryItem,
            AudioPickerTargetModel::OutputSampleRate => Self::PrimaryNumber,
            AudioPickerTargetModel::InputHost => Self::SecondaryGroup,
            AudioPickerTargetModel::InputDevice => Self::SecondaryItem,
            AudioPickerTargetModel::InputSampleRate => Self::SecondaryNumber,
        }
    }
}

impl From<compat::PairedPickerValueModel> for AudioOptionValueModel {
    fn from(value: compat::PairedPickerValueModel) -> Self {
        match value {
            compat::PairedPickerValueModel::PrimaryGroup(value) => Self::OutputHost(value),
            compat::PairedPickerValueModel::PrimaryItem(value) => Self::OutputDevice(value),
            compat::PairedPickerValueModel::PrimaryNumber(value) => Self::OutputSampleRate(value),
            compat::PairedPickerValueModel::SecondaryGroup(value) => Self::InputHost(value),
            compat::PairedPickerValueModel::SecondaryItem(value) => Self::InputDevice(value),
            compat::PairedPickerValueModel::SecondaryNumber(value) => Self::InputSampleRate(value),
        }
    }
}

impl From<AudioOptionValueModel> for compat::PairedPickerValueModel {
    fn from(value: AudioOptionValueModel) -> Self {
        match value {
            AudioOptionValueModel::OutputHost(value) => Self::PrimaryGroup(value),
            AudioOptionValueModel::OutputDevice(value) => Self::PrimaryItem(value),
            AudioOptionValueModel::OutputSampleRate(value) => Self::PrimaryNumber(value),
            AudioOptionValueModel::InputHost(value) => Self::SecondaryGroup(value),
            AudioOptionValueModel::InputDevice(value) => Self::SecondaryItem(value),
            AudioOptionValueModel::InputSampleRate(value) => Self::SecondaryNumber(value),
        }
    }
}

fn audio_option_item_from_compat(value: compat::PairedPickerOptionModel) -> AudioOptionItemModel {
    AudioOptionItemModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

fn audio_option_item_to_compat(value: AudioOptionItemModel) -> compat::PairedPickerOptionModel {
    compat::PairedPickerOptionModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

impl From<compat::PairedDevicePanelModel> for AudioEngineModel {
    fn from(value: compat::PairedDevicePanelModel) -> Self {
        Self {
            chip_state: value.status_state.into(),
            chip_label: value.status_label,
            detail_label: value.detail_label,
            output_host: value.primary_group.into(),
            output_device: value.primary_item.into(),
            output_sample_rate: value.primary_number.into(),
            input_host: value.secondary_group.into(),
            input_device: value.secondary_item.into(),
            input_sample_rate: value.secondary_number.into(),
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .primary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_device_options: value
                .primary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_sample_rate_options: value
                .primary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_host_options: value
                .secondary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_device_options: value
                .secondary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_sample_rate_options: value
                .secondary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
        }
    }
}

impl From<AudioEngineModel> for compat::PairedDevicePanelModel {
    fn from(value: AudioEngineModel) -> Self {
        Self {
            status_state: value.chip_state.into(),
            status_label: value.chip_label,
            detail_label: value.detail_label,
            primary_group: value.output_host.into(),
            primary_item: value.output_device.into(),
            primary_number: value.output_sample_rate.into(),
            secondary_group: value.input_host.into(),
            secondary_item: value.input_device.into(),
            secondary_number: value.input_sample_rate.into(),
            active_picker: value.active_picker.map(Into::into),
            primary_group_options: value
                .output_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_item_options: value
                .output_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_number_options: value
                .output_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_group_options: value
                .input_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_item_options: value
                .input_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_number_options: value
                .input_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
        }
    }
}

impl From<&AudioEngineModel> for compat::PairedDevicePanelModel {
    fn from(value: &AudioEngineModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::OptionsPanelModel> for OptionsPanelModel {
    fn from(value: compat::OptionsPanelModel) -> Self {
        Self {
            visible: value.visible,
            default_identifier: value.default_identifier,
            input_monitoring_enabled: value.input_monitoring_enabled,
            advance_after_rating_enabled: value.advance_after_rating_enabled,
            destructive_yolo_mode_enabled: value.destructive_yolo_mode_enabled,
            invert_waveform_scroll_enabled: value.invert_waveform_scroll_enabled,
            trash_folder_label: value.trash_folder_label,
        }
    }
}

impl From<OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: OptionsPanelModel) -> Self {
        Self {
            visible: value.visible,
            default_identifier: value.default_identifier,
            input_monitoring_enabled: value.input_monitoring_enabled,
            advance_after_rating_enabled: value.advance_after_rating_enabled,
            destructive_yolo_mode_enabled: value.destructive_yolo_mode_enabled,
            invert_waveform_scroll_enabled: value.invert_waveform_scroll_enabled,
            trash_folder_label: value.trash_folder_label,
        }
    }
}

impl From<&OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: &OptionsPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::ConfirmPromptKind> for ConfirmPromptKind {
    fn from(value: compat::ConfirmPromptKind) -> Self {
        match value {
            compat::ConfirmPromptKind::DestructiveOperation => Self::DestructiveEdit,
            compat::ConfirmPromptKind::RenameContent => Self::BrowserRename,
            compat::ConfirmPromptKind::RenameNavigationItem => Self::FolderRename,
            compat::ConfirmPromptKind::CreateNavigationItem => Self::FolderCreate,
            compat::ConfirmPromptKind::RestoreRetainedItems => Self::RestoreRetainedFolderDeletes,
            compat::ConfirmPromptKind::PurgeRetainedItems => Self::PurgeRetainedFolderDeletes,
            compat::ConfirmPromptKind::EditConfiguration => Self::OptionsDefaultIdentifier,
        }
    }
}

impl From<ConfirmPromptKind> for compat::ConfirmPromptKind {
    fn from(value: ConfirmPromptKind) -> Self {
        match value {
            ConfirmPromptKind::DestructiveEdit => Self::DestructiveOperation,
            ConfirmPromptKind::BrowserRename => Self::RenameContent,
            ConfirmPromptKind::FolderRename => Self::RenameNavigationItem,
            ConfirmPromptKind::FolderCreate => Self::CreateNavigationItem,
            ConfirmPromptKind::RestoreRetainedFolderDeletes => Self::RestoreRetainedItems,
            ConfirmPromptKind::PurgeRetainedFolderDeletes => Self::PurgeRetainedItems,
            ConfirmPromptKind::OptionsDefaultIdentifier => Self::EditConfiguration,
        }
    }
}

fn confirm_prompt_from_compat(value: compat::ConfirmPromptModel) -> ConfirmPromptModel {
    ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

fn confirm_prompt_to_compat(value: ConfirmPromptModel) -> compat::ConfirmPromptModel {
    compat::ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

impl From<compat::WaveformPanelModel> for WaveformPanelModel {
    fn from(value: compat::WaveformPanelModel) -> Self {
        Self {
            loaded_label: value.loaded_label,
            loading: value.loading,
            image_rendering: value.image_rendering,
            cursor_milli: value.cursor_milli,
            playhead_milli: value.playhead_milli,
            playhead_micros: value.playhead_micros,
            selection_milli: value.selection_milli.map(Into::into),
            slices: value.slices.into_iter().map(Into::into).collect(),
            selection_export_flash_nonce: value.selection_export_flash_nonce,
            selection_export_failure_flash_nonce: value.selection_export_failure_flash_nonce,
            edit_selection_apply_flash_nonce: value.edit_selection_apply_flash_nonce,
            edit_selection_milli: value.edit_selection_milli.map(Into::into),
            edit_fade_in_end_milli: value.edit_fade_in_end_milli,
            edit_fade_in_end_micros: value.edit_fade_in_end_micros,
            edit_fade_in_mute_start_milli: value.edit_fade_in_mute_start_milli,
            edit_fade_in_mute_start_micros: value.edit_fade_in_mute_start_micros,
            edit_fade_in_curve_milli: value.edit_fade_in_curve_milli,
            edit_fade_out_start_milli: value.edit_fade_out_start_milli,
            edit_fade_out_start_micros: value.edit_fade_out_start_micros,
            edit_fade_out_mute_end_milli: value.edit_fade_out_mute_end_milli,
            edit_fade_out_mute_end_micros: value.edit_fade_out_mute_end_micros,
            edit_fade_out_curve_milli: value.edit_fade_out_curve_milli,
            view_start_milli: value.view_start_milli,
            view_end_milli: value.view_end_milli,
            view_start_micros: value.view_start_micros,
            view_end_micros: value.view_end_micros,
            view_start_nanos: value.view_start_nanos,
            view_end_nanos: value.view_end_nanos,
            beat_step_micros: value.beat_step_micros,
            bpm_grid_origin_micros: value.bpm_grid_origin_micros,
            loop_enabled: value.loop_enabled,
            tempo_label: value.tempo_label,
            zoom_label: value.zoom_label,
            waveform_image_signature: value.waveform_image_signature,
            waveform_image: value.waveform_image,
        }
    }
}

impl From<WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: WaveformPanelModel) -> Self {
        Self {
            loaded_label: value.loaded_label,
            loading: value.loading,
            image_rendering: value.image_rendering,
            cursor_milli: value.cursor_milli,
            playhead_milli: value.playhead_milli,
            playhead_micros: value.playhead_micros,
            selection_milli: value.selection_milli.map(Into::into),
            slices: value.slices.into_iter().map(Into::into).collect(),
            selection_export_flash_nonce: value.selection_export_flash_nonce,
            selection_export_failure_flash_nonce: value.selection_export_failure_flash_nonce,
            edit_selection_apply_flash_nonce: value.edit_selection_apply_flash_nonce,
            edit_selection_milli: value.edit_selection_milli.map(Into::into),
            edit_fade_in_end_milli: value.edit_fade_in_end_milli,
            edit_fade_in_end_micros: value.edit_fade_in_end_micros,
            edit_fade_in_mute_start_milli: value.edit_fade_in_mute_start_milli,
            edit_fade_in_mute_start_micros: value.edit_fade_in_mute_start_micros,
            edit_fade_in_curve_milli: value.edit_fade_in_curve_milli,
            edit_fade_out_start_milli: value.edit_fade_out_start_milli,
            edit_fade_out_start_micros: value.edit_fade_out_start_micros,
            edit_fade_out_mute_end_milli: value.edit_fade_out_mute_end_milli,
            edit_fade_out_mute_end_micros: value.edit_fade_out_mute_end_micros,
            edit_fade_out_curve_milli: value.edit_fade_out_curve_milli,
            view_start_milli: value.view_start_milli,
            view_end_milli: value.view_end_milli,
            view_start_micros: value.view_start_micros,
            view_end_micros: value.view_end_micros,
            view_start_nanos: value.view_start_nanos,
            view_end_nanos: value.view_end_nanos,
            beat_step_micros: value.beat_step_micros,
            bpm_grid_origin_micros: value.bpm_grid_origin_micros,
            loop_enabled: value.loop_enabled,
            tempo_label: value.tempo_label,
            zoom_label: value.zoom_label,
            waveform_image_signature: value.waveform_image_signature,
            waveform_image: value.waveform_image,
        }
    }
}

impl From<&WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: &WaveformPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::WaveformChromeModel> for WaveformChromeModel {
    fn from(value: compat::WaveformChromeModel) -> Self {
        Self {
            transport_hint: value.transport_hint,
            compare_anchor_available: value.compare_anchor_available,
            compare_anchor_label: value.compare_anchor_label,
            loop_lock_enabled: value.loop_lock_enabled,
            channel_view: value.channel_view.into(),
            normalized_audition_enabled: value.normalized_audition_enabled,
            bpm_snap_enabled: value.bpm_snap_enabled,
            relative_bpm_grid_enabled: value.relative_bpm_grid_enabled,
            transient_snap_enabled: value.transient_snap_enabled,
            transient_markers_enabled: value.transient_markers_enabled,
            slice_mode_enabled: value.slice_mode_enabled,
            exact_duplicate_cleanup_available: value.exact_duplicate_cleanup_available,
        }
    }
}

impl From<WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: WaveformChromeModel) -> Self {
        Self {
            transport_hint: value.transport_hint,
            compare_anchor_available: value.compare_anchor_available,
            compare_anchor_label: value.compare_anchor_label,
            loop_lock_enabled: value.loop_lock_enabled,
            channel_view: value.channel_view.into(),
            normalized_audition_enabled: value.normalized_audition_enabled,
            bpm_snap_enabled: value.bpm_snap_enabled,
            relative_bpm_grid_enabled: value.relative_bpm_grid_enabled,
            transient_snap_enabled: value.transient_snap_enabled,
            transient_markers_enabled: value.transient_markers_enabled,
            slice_mode_enabled: value.slice_mode_enabled,
            exact_duplicate_cleanup_available: value.exact_duplicate_cleanup_available,
        }
    }
}

impl From<&WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: &WaveformChromeModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::AppModel> for AppModel {
    fn from(value: compat::AppModel) -> Self {
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status.into(),
            audio_engine: value.paired_device.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: confirm_prompt_from_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay.into(),
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources.into(),
            browser: value.browser.into(),
            browser_chrome: value.browser_chrome.into(),
            map: value.map.into(),
            waveform: value.waveform.into(),
            waveform_chrome: value.waveform_chrome.into(),
            update: value.update.into(),
            focus_context: value.focus_context.into(),
        }
    }
}

impl From<AppModel> for compat::AppModel {
    fn from(value: AppModel) -> Self {
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status.into(),
            paired_device: value.audio_engine.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: confirm_prompt_to_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay.into(),
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources.into(),
            browser: value.browser.into(),
            browser_chrome: value.browser_chrome.into(),
            map: value.map.into(),
            waveform: value.waveform.into(),
            waveform_chrome: value.waveform_chrome.into(),
            update: value.update.into(),
            focus_context: value.focus_context.into(),
        }
    }
}

impl From<&AppModel> for compat::AppModel {
    fn from(value: &AppModel) -> Self {
        value.clone().into()
    }
}

fn local_app_model_from_native_model(value: &AppModel) -> crate::compat_app_contract::AppModel {
    crate::compat_app_contract::AppModel {
        title: value.title.clone(),
        backend_label: value.backend_label.clone(),
        sources_label: value.sources_label.clone(),
        status_text: value.status_text.clone(),
        status: value.status.clone(),
        paired_device: value.audio_engine.clone().into(),
        browser_actions: value.browser_actions.clone().into(),
        options_panel: local_options_panel_from_native_model(&value.options_panel),
        progress_overlay: value.progress_overlay.clone(),
        confirm_prompt: local_confirm_prompt_from_native_model(&value.confirm_prompt),
        drag_overlay: value.drag_overlay.clone(),
        columns: value.columns.clone().map(Into::into),
        selected_column: value.selected_column,
        volume: value.volume,
        transport_running: value.transport_running,
        sources: local_sources_panel_from_native_model(&value.sources),
        browser: value.browser.clone().into(),
        browser_chrome: value.browser_chrome.clone().into(),
        map: value.map.clone(),
        waveform: local_waveform_panel_from_native_model(&value.waveform),
        waveform_chrome: local_waveform_chrome_from_native_model(&value.waveform_chrome),
        update: value.update.clone(),
        focus_context: value.focus_context.into(),
    }
}

fn local_options_panel_from_native_model(
    value: &OptionsPanelModel,
) -> crate::compat_app_contract::OptionsPanelModel {
    crate::compat_app_contract::OptionsPanelModel {
        visible: value.visible,
        default_identifier: value.default_identifier.clone(),
        input_monitoring_enabled: value.input_monitoring_enabled,
        advance_after_rating_enabled: value.advance_after_rating_enabled,
        destructive_yolo_mode_enabled: value.destructive_yolo_mode_enabled,
        invert_waveform_scroll_enabled: value.invert_waveform_scroll_enabled,
        trash_folder_label: value.trash_folder_label.clone(),
    }
}

fn local_confirm_prompt_from_native_model(
    value: &ConfirmPromptModel,
) -> crate::compat_app_contract::ConfirmPromptModel {
    crate::compat_app_contract::ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title.clone(),
        message: value.message.clone(),
        confirm_label: value.confirm_label.clone(),
        cancel_label: value.cancel_label.clone(),
        target_label: value.target_label.clone(),
        input_value: value.input_value.clone(),
        input_placeholder: value.input_placeholder.clone(),
        input_error: value.input_error.clone(),
    }
}

fn local_sources_panel_from_native_model(
    value: &SourcesPanelModel,
) -> crate::compat_app_contract::SourcesPanelModel {
    crate::compat_app_contract::SourcesPanelModel {
        header: value.header.clone(),
        search_query: value.search_query.clone(),
        active_folder_pane: value.active_folder_pane,
        upper_folder_pane: value.upper_folder_pane.clone(),
        lower_folder_pane: value.lower_folder_pane.clone(),
        tree_search_query: value.tree_search_query.clone(),
        show_all_items: value.show_all_items,
        can_toggle_show_all_items: value.can_toggle_show_all_items,
        flattened_view: value.flattened_view,
        can_toggle_flattened_view: value.can_toggle_flattened_view,
        selected_row: value.selected_row,
        loading_row: value.loading_row,
        mutation_busy_row: value.mutation_busy_row,
        focused_tree_row: value.focused_tree_row,
        rows: value.rows.clone(),
        tree_rows: value.tree_rows.clone(),
        tree_actions: value.tree_actions.clone(),
        recovery: value.recovery.clone(),
    }
}

fn local_waveform_panel_from_native_model(
    value: &WaveformPanelModel,
) -> crate::compat_app_contract::WaveformPanelModel {
    crate::compat_app_contract::WaveformPanelModel {
        loaded_label: value.loaded_label.clone(),
        loading: value.loading,
        image_rendering: value.image_rendering,
        cursor_milli: value.cursor_milli,
        playhead_milli: value.playhead_milli,
        playhead_micros: value.playhead_micros,
        selection_milli: value.selection_milli,
        slices: value.slices.clone(),
        selection_export_flash_nonce: value.selection_export_flash_nonce,
        selection_export_failure_flash_nonce: value.selection_export_failure_flash_nonce,
        edit_selection_apply_flash_nonce: value.edit_selection_apply_flash_nonce,
        edit_selection_milli: value.edit_selection_milli,
        edit_fade_in_end_milli: value.edit_fade_in_end_milli,
        edit_fade_in_end_micros: value.edit_fade_in_end_micros,
        edit_fade_in_mute_start_milli: value.edit_fade_in_mute_start_milli,
        edit_fade_in_mute_start_micros: value.edit_fade_in_mute_start_micros,
        edit_fade_in_curve_milli: value.edit_fade_in_curve_milli,
        edit_fade_out_start_milli: value.edit_fade_out_start_milli,
        edit_fade_out_start_micros: value.edit_fade_out_start_micros,
        edit_fade_out_mute_end_milli: value.edit_fade_out_mute_end_milli,
        edit_fade_out_mute_end_micros: value.edit_fade_out_mute_end_micros,
        edit_fade_out_curve_milli: value.edit_fade_out_curve_milli,
        view_start_milli: value.view_start_milli,
        view_end_milli: value.view_end_milli,
        view_start_micros: value.view_start_micros,
        view_end_micros: value.view_end_micros,
        view_start_nanos: value.view_start_nanos,
        view_end_nanos: value.view_end_nanos,
        beat_step_micros: value.beat_step_micros,
        bpm_grid_origin_micros: value.bpm_grid_origin_micros,
        loop_enabled: value.loop_enabled,
        tempo_label: value.tempo_label.clone(),
        zoom_label: value.zoom_label.clone(),
        waveform_image_signature: value.waveform_image_signature,
        waveform_image: value.waveform_image.clone(),
    }
}

fn local_waveform_chrome_from_native_model(
    value: &WaveformChromeModel,
) -> crate::compat_app_contract::WaveformChromeModel {
    crate::compat_app_contract::WaveformChromeModel {
        transport_hint: value.transport_hint.clone(),
        compare_anchor_available: value.compare_anchor_available,
        compare_anchor_label: value.compare_anchor_label.clone(),
        loop_lock_enabled: value.loop_lock_enabled,
        channel_view: value.channel_view.into(),
        normalized_audition_enabled: value.normalized_audition_enabled,
        bpm_snap_enabled: value.bpm_snap_enabled,
        relative_bpm_grid_enabled: value.relative_bpm_grid_enabled,
        transient_snap_enabled: value.transient_snap_enabled,
        transient_markers_enabled: value.transient_markers_enabled,
        slice_mode_enabled: value.slice_mode_enabled,
        exact_duplicate_cleanup_available: value.exact_duplicate_cleanup_available,
    }
}

fn automation_node_id_from_generic(value: gui_automation::AutomationNodeId) -> AutomationNodeId {
    gui_automation::AutomationNodeId(automation_node_id_string_from_generic(value.0))
}

fn automation_node_id_string_from_generic(node_id: String) -> String {
    match node_id.as_str() {
        "browser.tab.items" => String::from("browser.tab.samples"),
        "browser.pill_editor" => String::from("browser.tag_sidebar"),
        "browser.pill_editor.input" => String::from("browser.tag_sidebar.input"),
        "browser.pill_editor.exclusive.0" => String::from("browser.tag_sidebar.playback.loop"),
        "browser.pill_editor.exclusive.1" => String::from("browser.tag_sidebar.playback.one_shot"),
        _ => {
            if let Some(suffix) = node_id.strip_prefix("browser.pill_editor.option.") {
                format!("browser.tag_sidebar.normal_tag.{suffix}")
            } else if let Some(suffix) = node_id.strip_prefix("browser.pill_editor.create.") {
                format!("browser.tag_sidebar.create_tag.{suffix}")
            } else {
                node_id
            }
        }
    }
}

impl From<gui_automation::AutomationRole> for AutomationRole {
    fn from(value: gui_automation::AutomationRole) -> Self {
        match value {
            gui_automation::AutomationRole::Root => Self::Root,
            gui_automation::AutomationRole::Group => Self::Group,
            gui_automation::AutomationRole::Panel => Self::Panel,
            gui_automation::AutomationRole::Toolbar => Self::Toolbar,
            gui_automation::AutomationRole::TabList => Self::TabList,
            gui_automation::AutomationRole::Tab => Self::Tab,
            gui_automation::AutomationRole::Button => Self::Button,
            gui_automation::AutomationRole::SearchField => Self::SearchField,
            gui_automation::AutomationRole::Slider => Self::Slider,
            gui_automation::AutomationRole::Row => Self::Row,
            gui_automation::AutomationRole::Table => Self::Table,
            gui_automation::AutomationRole::TimelineRegion => Self::WaveformRegion,
            gui_automation::AutomationRole::SpatialCanvas => Self::MapCanvas,
            gui_automation::AutomationRole::SpatialPoint => Self::MapPoint,
            gui_automation::AutomationRole::Readout => Self::Readout,
            gui_automation::AutomationRole::Dialog => Self::Dialog,
        }
    }
}

fn automation_bounds_from_generic(value: gui_automation::AutomationBounds) -> AutomationBounds {
    AutomationBounds {
        x: value.x,
        y: value.y,
        width: value.width,
        height: value.height,
    }
}

impl From<gui_automation::AutomationNodeSnapshot> for AutomationNodeSnapshot {
    fn from(value: gui_automation::AutomationNodeSnapshot) -> Self {
        Self {
            id: automation_node_id_from_generic(value.id),
            role: value.role.into(),
            label: value.label,
            bounds: automation_bounds_from_generic(value.bounds),
            value: value.value,
            enabled: value.enabled,
            selected: value.selected,
            available_actions: value
                .available_actions
                .into_iter()
                .map(automation_action_id_from_generic)
                .collect(),
            metadata: automation_metadata_from_generic(value.metadata),
            children: value.children.into_iter().map(Into::into).collect(),
        }
    }
}

fn automation_action_id_from_generic(action_id: String) -> String {
    match action_id.as_str() {
        "open_primary_group_picker" => String::from("open_audio_output_host_picker"),
        "open_primary_item_picker" => String::from("open_audio_output_device_picker"),
        "open_primary_number_picker" => String::from("open_audio_output_sample_rate_picker"),
        "open_secondary_group_picker" => String::from("open_audio_input_host_picker"),
        "open_secondary_item_picker" => String::from("open_audio_input_device_picker"),
        "open_secondary_number_picker" => String::from("open_audio_input_sample_rate_picker"),
        "set_primary_group" => String::from("set_audio_output_host"),
        "set_primary_item" => String::from("set_audio_output_device"),
        "set_primary_number" => String::from("set_audio_output_sample_rate"),
        "set_secondary_group" => String::from("set_audio_input_host"),
        "set_secondary_item" => String::from("set_audio_input_device"),
        "set_secondary_number" => String::from("set_audio_input_sample_rate"),
        "focus_spatial_content_item" => String::from("focus_map_sample"),
        "focus_browser_pill_editor_input" => String::from("focus_browser_tag_sidebar_input"),
        "set_browser_pill_editor_input" => String::from("set_browser_tag_sidebar_input"),
        "commit_browser_pill_editor_input" => String::from("commit_browser_tag_sidebar_input"),
        "toggle_browser_pill_editor" => String::from("toggle_browser_tag_sidebar"),
        "toggle_browser_pill_editor_primary_action" => {
            String::from("toggle_browser_tag_sidebar_auto_rename")
        }
        "toggle_browser_pill_option" => String::from("toggle_browser_sidebar_normal_tag"),
        "toggle_browser_derived_label_filter" => String::from("toggle_browser_tag_named_filter"),
        _ => action_id,
    }
}

fn automation_metadata_from_generic(
    mut metadata: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if let Some(value) = metadata.remove("focused_item_label") {
        metadata.insert(String::from("focused_sample_label"), value);
    }
    if let Some(value) = metadata.remove("option_pill_labels") {
        metadata.insert(String::from("normal_tag_labels"), value);
    }
    if let Some(value) = metadata.remove("pill_state") {
        metadata.insert(String::from("tag_state"), value);
    }
    if let Some(value) = metadata.remove("pill_id") {
        metadata.insert(String::from("tag_id"), value);
    }
    metadata
}

impl From<gui_automation::GuiAutomationSnapshot> for GuiAutomationSnapshot {
    fn from(value: gui_automation::GuiAutomationSnapshot) -> Self {
        Self {
            schema_version: value.schema_version,
            viewport_width: value.viewport_width,
            viewport_height: value.viewport_height,
            root: value.root.into(),
        }
    }
}

impl From<compat::DirtySegments> for DirtySegments {
    fn from(value: compat::DirtySegments) -> Self {
        Self::from_bits(value.bits())
    }
}

impl From<DirtySegments> for compat::DirtySegments {
    fn from(value: DirtySegments) -> Self {
        Self::from_bits(value.bits())
    }
}

impl From<compat::SegmentRevisions> for SegmentRevisions {
    fn from(value: compat::SegmentRevisions) -> Self {
        Self {
            status_bar: value.status_bar,
            browser_frame: value.browser_frame,
            browser_rows_window: value.browser_rows_window,
            map_panel: value.map_panel,
            waveform_overlay: value.waveform_overlay,
            global_static: value.global_static,
        }
    }
}

impl From<SegmentRevisions> for compat::SegmentRevisions {
    fn from(value: SegmentRevisions) -> Self {
        Self {
            status_bar: value.status_bar,
            browser_frame: value.browser_frame,
            browser_rows_window: value.browser_rows_window,
            map_panel: value.map_panel,
            waveform_overlay: value.waveform_overlay,
            global_static: value.global_static,
        }
    }
}

impl From<compat::NativeMotionModel> for NativeMotionModel {
    fn from(value: compat::NativeMotionModel) -> Self {
        Self {
            transport_running: value.transport_running,
            map_active: value.map_active,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            waveform_selection_milli: value.waveform_selection_milli.map(Into::into),
            waveform_slices: value.waveform_slices.into_iter().map(Into::into).collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli.map(Into::into),
            waveform_edit_fade_in_end_milli: value.waveform_edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: value.waveform_edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: value.waveform_edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: value.waveform_edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: value.waveform_edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: value.waveform_edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: value.waveform_edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: value.waveform_edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: value.waveform_edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: value.waveform_edit_fade_out_curve_milli,
            waveform_loop_enabled: value.waveform_loop_enabled,
            waveform_loop_lock_enabled: value.waveform_loop_lock_enabled,
            waveform_cursor_milli: value.waveform_cursor_milli,
            waveform_playhead_milli: value.waveform_playhead_milli,
            waveform_playhead_micros: value.waveform_playhead_micros,
            waveform_view_start_milli: value.waveform_view_start_milli,
            waveform_view_end_milli: value.waveform_view_end_milli,
            waveform_view_start_micros: value.waveform_view_start_micros,
            waveform_view_end_micros: value.waveform_view_end_micros,
            waveform_view_start_nanos: value.waveform_view_start_nanos,
            waveform_view_end_nanos: value.waveform_view_end_nanos,
            waveform_tempo_label: value.waveform_tempo_label,
            waveform_zoom_label: value.waveform_zoom_label,
            waveform_loaded_label: value.waveform_loaded_label,
            waveform_loading: value.waveform_loading,
            waveform_image_signature: value.waveform_image_signature,
            waveform_transport_hint: value.waveform_transport_hint,
            waveform_compare_anchor_available: value.waveform_compare_anchor_available,
            waveform_compare_anchor_label: value.waveform_compare_anchor_label,
            waveform_channel_view: value.waveform_channel_view.into(),
            waveform_normalized_audition_enabled: value.waveform_normalized_audition_enabled,
            waveform_bpm_snap_enabled: value.waveform_bpm_snap_enabled,
            waveform_relative_bpm_grid_enabled: value.waveform_relative_bpm_grid_enabled,
            waveform_transient_snap_enabled: value.waveform_transient_snap_enabled,
            waveform_transient_markers_enabled: value.waveform_transient_markers_enabled,
            waveform_slice_mode_enabled: value.waveform_slice_mode_enabled,
            waveform_exact_duplicate_cleanup_available: value
                .waveform_exact_duplicate_cleanup_available,
            status_right: value.status_right,
        }
    }
}

impl From<NativeMotionModel> for compat::NativeMotionModel {
    fn from(value: NativeMotionModel) -> Self {
        Self {
            transport_running: value.transport_running,
            map_active: value.map_active,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            waveform_selection_milli: value.waveform_selection_milli.map(Into::into),
            waveform_slices: value.waveform_slices.into_iter().map(Into::into).collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli.map(Into::into),
            waveform_edit_fade_in_end_milli: value.waveform_edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: value.waveform_edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: value.waveform_edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: value.waveform_edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: value.waveform_edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: value.waveform_edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: value.waveform_edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: value.waveform_edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: value.waveform_edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: value.waveform_edit_fade_out_curve_milli,
            waveform_loop_enabled: value.waveform_loop_enabled,
            waveform_loop_lock_enabled: value.waveform_loop_lock_enabled,
            waveform_cursor_milli: value.waveform_cursor_milli,
            waveform_playhead_milli: value.waveform_playhead_milli,
            waveform_playhead_micros: value.waveform_playhead_micros,
            waveform_view_start_milli: value.waveform_view_start_milli,
            waveform_view_end_milli: value.waveform_view_end_milli,
            waveform_view_start_micros: value.waveform_view_start_micros,
            waveform_view_end_micros: value.waveform_view_end_micros,
            waveform_view_start_nanos: value.waveform_view_start_nanos,
            waveform_view_end_nanos: value.waveform_view_end_nanos,
            waveform_tempo_label: value.waveform_tempo_label,
            waveform_zoom_label: value.waveform_zoom_label,
            waveform_loaded_label: value.waveform_loaded_label,
            waveform_loading: value.waveform_loading,
            waveform_image_signature: value.waveform_image_signature,
            waveform_transport_hint: value.waveform_transport_hint,
            waveform_compare_anchor_available: value.waveform_compare_anchor_available,
            waveform_compare_anchor_label: value.waveform_compare_anchor_label,
            waveform_channel_view: value.waveform_channel_view.into(),
            waveform_normalized_audition_enabled: value.waveform_normalized_audition_enabled,
            waveform_bpm_snap_enabled: value.waveform_bpm_snap_enabled,
            waveform_relative_bpm_grid_enabled: value.waveform_relative_bpm_grid_enabled,
            waveform_transient_snap_enabled: value.waveform_transient_snap_enabled,
            waveform_transient_markers_enabled: value.waveform_transient_markers_enabled,
            waveform_slice_mode_enabled: value.waveform_slice_mode_enabled,
            waveform_exact_duplicate_cleanup_available: value
                .waveform_exact_duplicate_cleanup_available,
            status_right: value.status_right,
        }
    }
}

pub(super) fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    run_native_vello_app_with_artifacts(options, bridge).result
}

pub(super) fn run_native_vello_app_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    let report = radiant::gui_runtime::run_legacy_native_vello_app_with_artifacts(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    );
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

pub(super) fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    let local_model = local_app_model_from_native_model(model);
    let viewport = Vector2::new(viewport[0].max(1.0), viewport[1].max(1.0));
    let style = StyleTokens::for_viewport_width(viewport.x);
    let mut runtime = ShellLayoutRuntime::default();
    let layout = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
    let mut shell_state = NativeShellState::new();
    shell_state.sync_from_model(&local_model);
    shell_state
        .automation_snapshot(&layout, &local_model)
        .into()
}

#[cfg(test)]
pub(super) fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> impl serde::Serialize {
    let local_model = local_app_model_from_native_model(model);
    crate::compat_app_contract::capture_native_shell_shot_snapshot(name, viewport, &local_model)
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

        let compat: radiant::gui_runtime::NativeRunOptions = options.into();

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

    #[test]
    fn automation_snapshot_uses_local_sempal_native_shell() {
        let source = include_str!("native_vello.rs");
        let function_start = source
            .find("pub(super) fn capture_gui_automation_snapshot")
            .expect("snapshot adapter function should exist");
        let function_body = &source[function_start..];
        let function_end = function_body
            .find("#[cfg(test)]")
            .expect("test-only snapshot adapter should follow automation adapter");
        let function_body = &function_body[..function_end];

        assert!(function_body.contains("local_app_model_from_native_model"));
        assert!(!function_body.contains("radiant::compat::legacy_shell::AppModel"));
        let removed_helper = concat!("local_app_model_from_", "radiant_compat");
        assert!(!source.contains(removed_helper));
        assert!(function_body.contains("NativeShellState::new()"));
        assert!(
            !function_body
                .contains("radiant::compat::legacy_shell::capture_gui_automation_snapshot"),
            "automation snapshots should be generated by Sempal's local native shell scaffold"
        );
    }

    #[test]
    fn native_shell_shot_snapshot_uses_local_sempal_native_shell() {
        let source = include_str!("native_vello.rs");
        let function_start = source
            .find("pub(super) fn capture_native_shell_shot_snapshot")
            .expect("shot snapshot adapter function should exist");
        let function_body = &source[function_start..];
        let function_end = function_body
            .find("#[cfg(test)]")
            .expect("tests should follow shot snapshot adapter");
        let function_body = &function_body[..function_end];

        assert!(function_body.contains("local_app_model_from_native_model"));
        assert!(!function_body.contains("radiant::compat::legacy_shell::AppModel"));
        let removed_helper = concat!("local_app_model_from_", "radiant_compat");
        assert!(!source.contains(removed_helper));
        assert!(
            function_body
                .contains("crate::compat_app_contract::capture_native_shell_shot_snapshot")
        );
        assert!(
            !function_body
                .contains("radiant::compat::legacy_shell::capture_native_shell_shot_snapshot"),
            "native shell shot snapshots should be generated by Sempal's local native shell scaffold"
        );
    }
}
