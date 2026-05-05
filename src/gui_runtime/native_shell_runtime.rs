//! Sempal native-shell adapter for the temporary local legacy runtime path.
//!
//! This module owns Sempal compatibility DTO conversion, automation snapshot
//! mapping, and launch handoff to Sempal's local native-Vello runner. It should
//! not call Radiant's legacy-shell facade; OPT-277 owns the later cutover to
//! Radiant's generic runtime bridge.

use super::{NativeRunOptions, NativeRunReport, NativeRuntimeArtifacts, WindowIconRgba};
use crate::app_core::actions::{
    NativeAppBridge, NativeAppModel, NativeBrowserTagTarget as BrowserTagTarget,
    NativeFrameBuildResult, NativeGuiAutomationSnapshot, NativeMotionModel, NativeUiAction,
    NativeUiAction as UiAction, native_shell_dtos::*,
};
use crate::app_core::app_api::controller_ui_hotkeys::KeyPress;
use crate::app_core::app_api::{controller_ui_hotkeys as hotkeys, state::FocusContext};
use crate::compat_app_contract as compat;
use crate::gui::automation as gui_automation;
use crate::gui::{
    native_shell::{NativeShellState, ShellLayout, ShellLayoutRuntime, StyleTokens},
    types::Vector2,
};
use crate::gui_runtime::native_vello;
use radiant::gui::{
    focus::FocusSurface as RadiantFocusSurface, frame::FrameBuildResult as RadiantFrameBuildResult,
    input::KeyPress as RadiantKeyPress, shortcuts::ShortcutResolution as RadiantShortcutResolution,
};
use radiant::runtime::{RuntimeBridge, SurfaceNode, UiSurface};
use radiant::widgets::{CanvasMessage, WidgetSizing};
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

/// Sempal-owned generic Radiant runtime bridge.
///
/// The live native launch still uses Sempal's local Vello runner until the full
/// shell paint/input surface is represented by generic public widgets. This
/// bridge is the ownership boundary for the OPT-277 cutover: Sempal model
/// projection, action reduction, shortcut resolution, repaint wiring, and
/// shutdown artifacts no longer require Radiant's legacy-shell feature.
#[allow(dead_code)]
pub(super) struct SempalRuntimeBridge<B> {
    inner: B,
}

#[allow(dead_code)]
impl<B> SempalRuntimeBridge<B> {
    pub(super) fn new(inner: B) -> Self {
        Self { inner }
    }

    fn generic_shell_surface() -> Arc<UiSurface<UiAction>> {
        Arc::new(UiSurface::new(SurfaceNode::canvas_mapped(
            1,
            WidgetSizing::fixed(Vector2::new(1280.0, 720.0)),
            |_message: CanvasMessage| UiAction::HandleEscape,
        )))
    }

    pub(super) fn capture_gui_automation_snapshot(
        &mut self,
        viewport: [f32; 2],
    ) -> NativeGuiAutomationSnapshot
    where
        B: NativeAppBridge,
    {
        let model = self.inner.project_model();
        capture_gui_automation_snapshot(viewport, model.as_ref())
    }
}

impl<B: NativeAppBridge> RuntimeBridge<UiAction> for SempalRuntimeBridge<B> {
    fn project_surface(&mut self) -> Arc<UiSurface<UiAction>> {
        Self::generic_shell_surface()
    }

    fn reduce_message(&mut self, message: UiAction) {
        self.inner.reduce_action(message);
    }

    fn resolve_key_press(
        &mut self,
        pending_chord: Option<RadiantKeyPress>,
        press: RadiantKeyPress,
        focus: RadiantFocusSurface,
    ) -> RadiantShortcutResolution<UiAction> {
        let resolution = hotkeys::resolve_hotkey_press(
            pending_chord.map(keypress_from_radiant),
            keypress_from_radiant(press),
            focus_context_from_radiant(focus),
        );
        RadiantShortcutResolution {
            action: resolution.action,
            handled: resolution.handled,
            pending_chord: resolution.pending_chord.map(keypress_to_radiant),
        }
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.inner.install_repaint_signal(signal);
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.inner
            .on_runtime_exit()
            .and_then(|artifact| serde_json::to_value(artifact).ok())
    }
}

impl<B: NativeAppBridge> compat::NativeAppBridge for CompatNativeAppBridge<B> {
    fn project_model(&mut self) -> Arc<compat::AppModel> {
        let model = self.inner.project_model();
        Arc::new(model.as_ref().into())
    }

    fn pull_model(&mut self) -> compat::AppModel {
        self.inner.pull_model().into()
    }

    fn pull_model_arc(&mut self) -> Arc<compat::AppModel> {
        let model = self.inner.pull_model_arc();
        Arc::new(model.as_ref().into())
    }

    fn project_motion_model(&mut self) -> Option<compat::NativeMotionModel> {
        self.inner
            .project_motion_model()
            .map(NativeMotionModel::into)
    }

    fn take_dirty_segments(&mut self) -> compat::DirtySegments {
        self.inner.take_dirty_segments()
    }

    fn take_segment_revisions(&mut self) -> compat::SegmentRevisions {
        self.inner.take_segment_revisions()
    }

    fn resolve_hotkey_press(
        &mut self,
        pending_chord: Option<RadiantKeyPress>,
        press: RadiantKeyPress,
        focus: RadiantFocusSurface,
    ) -> RadiantShortcutResolution<compat::UiAction> {
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

    fn reduce_action(&mut self, action: compat::UiAction) {
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
            compat::UiAction::SelectColumn { index } => Self::SelectColumn { index },
            compat::UiAction::MoveColumn { delta } => Self::MoveColumn { delta },
            compat::UiAction::ToggleTransport => Self::ToggleTransport,
            compat::UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            compat::UiAction::PlayFromStart => Self::PlayFromStart,
            compat::UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            compat::UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            compat::UiAction::PlayWaveformAtPrecise { position_nanos } => {
                Self::PlayWaveformAtPrecise { position_nanos }
            }
            compat::UiAction::HandleEscape => Self::HandleEscape,
            compat::UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            compat::UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            compat::UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            compat::UiAction::FocusFolderPanel => Self::FocusFolderPanel,
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
            compat::UiAction::FocusFolderSearch => Self::FocusFolderSearch,
            compat::UiAction::SetFolderSearch { query } => Self::SetFolderSearch { query },
            compat::UiAction::ToggleShowAllFolders => Self::ToggleShowAllFolders,
            compat::UiAction::ToggleFolderFlattenedView => Self::ToggleFolderFlattenedView,
            compat::UiAction::FocusSourceRow { index } => Self::FocusSourceRow { index },
            compat::UiAction::SelectSourceRow { index } => Self::SelectSourceRow { index },
            compat::UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta },
            compat::UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            compat::UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            compat::UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            compat::UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            compat::UiAction::ReloadSourceRow { index } => Self::ReloadSourceRow { index },
            compat::UiAction::HardSyncSourceRow { index } => Self::HardSyncSourceRow { index },
            compat::UiAction::OpenSourceFolderRow { index } => Self::OpenSourceFolderRow { index },
            compat::UiAction::RemoveSourceRow { index } => Self::RemoveSourceRow { index },
            compat::UiAction::FocusFolderRow { index } => Self::FocusFolderRow { index },
            compat::UiAction::ActivateFolderRow { index } => Self::ActivateFolderRow { index },
            compat::UiAction::ToggleFolderRowExpanded { index } => {
                Self::ToggleFolderRowExpanded { index }
            }
            compat::UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            compat::UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            compat::UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            compat::UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta },
            compat::UiAction::StartNewFolder => Self::StartNewFolder,
            compat::UiAction::StartNewFolderAtFolderRow { index } => {
                Self::StartNewFolderAtFolderRow { index }
            }
            compat::UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            compat::UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            compat::UiAction::SetFolderCreateInput { value } => {
                Self::SetFolderCreateInput { value }
            }
            compat::UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            compat::UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            compat::UiAction::StartFolderRename => Self::StartFolderRename,
            compat::UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            compat::UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            compat::UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            compat::UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            compat::UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta },
            compat::UiAction::SetBrowserViewStart { visible_row } => {
                Self::SetBrowserViewStart { visible_row }
            }
            compat::UiAction::FocusBrowserRow { visible_row } => {
                Self::FocusBrowserRow { visible_row }
            }
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
                Self::ToggleBrowserRowSelection { visible_row }
            }
            compat::UiAction::StartContentItemDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
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
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            },
            compat::UiAction::FinishContentItemDrag => Self::FinishBrowserSampleDrag,
            compat::UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow { visible_row }
            }
            compat::UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeBrowserSelection { visible_row }
            }
            compat::UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta }
            }
            compat::UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta }
            }
            compat::UiAction::ToggleFocusedBrowserRowSelection => {
                Self::ToggleFocusedBrowserRowSelection
            }
            compat::UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            compat::UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query },
            compat::UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter { level, invert }
            }
            compat::UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter { bucket, invert }
            }
            compat::UiAction::ToggleContentMark => Self::ToggleBrowserSampleMark,
            compat::UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            compat::UiAction::ToggleBrowserDerivedLabelFilter { invert } => {
                Self::ToggleBrowserTagNamedFilter { invert }
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
                Self::ToggleBrowserDuplicateCleanupKeep { visible_row }
            }
            compat::UiAction::ConfirmBrowserDuplicateCleanup => {
                Self::ConfirmBrowserDuplicateCleanup
            }
            compat::UiAction::PlayRandomContentItem => Self::PlayRandomSample,
            compat::UiAction::PlayPreviousRandomContentItem => Self::PlayPreviousRandomSample,
            compat::UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta }
            }
            compat::UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map },
            compat::UiAction::FocusBrowserPillEditorInput => Self::FocusBrowserTagSidebarInput,
            compat::UiAction::SetBrowserPillEditorInput { value } => {
                Self::SetBrowserTagSidebarInput { value }
            }
            compat::UiAction::CommitBrowserPillEditorInput => Self::CommitBrowserTagSidebarInput,
            compat::UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped }
            }
            compat::UiAction::ToggleBrowserPillOption { label } => {
                Self::ToggleBrowserSidebarNormalTag { label }
            }
            compat::UiAction::FocusSpatialContentItem { content_id } => Self::FocusMapSample {
                sample_id: content_id,
            },
            compat::UiAction::SetPromptInput { value } => Self::SetPromptInput { value },
            compat::UiAction::StartBrowserRename => Self::StartBrowserRename,
            compat::UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            compat::UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            compat::UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection { visible_row }
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
                Self::ToggleWaveformSliceSelection { index }
            }
            compat::UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index }
            }
            compat::UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index }
            }
            compat::UiAction::MoveWaveformSliceFocus { delta } => {
                Self::MoveWaveformSliceFocus { delta }
            }
            compat::UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            compat::UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            compat::UiAction::DeleteLoadedWaveformContent => Self::DeleteLoadedWaveformSample,
            compat::UiAction::SlideWaveformSelection { delta, fine } => {
                Self::SlideWaveformSelection { delta, fine }
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
                Self::SetInputMonitoringEnabled { enabled }
            }
            compat::UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled }
            }
            compat::UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled }
            }
            compat::UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled }
            }
            compat::UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            compat::UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            compat::UiAction::SetWaveformChannelView { stereo } => {
                Self::SetWaveformChannelView { stereo }
            }
            compat::UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled }
            }
            compat::UiAction::SetBpmSnapEnabled { enabled } => Self::SetBpmSnapEnabled { enabled },
            compat::UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled }
            }
            compat::UiAction::AdjustWaveformBpm { delta } => Self::AdjustWaveformBpm { delta },
            compat::UiAction::SetWaveformBpmValue { value_tenths } => {
                Self::SetWaveformBpmValue { value_tenths }
            }
            compat::UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled }
            }
            compat::UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled }
            }
            compat::UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            compat::UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            compat::UiAction::SetSliceModeEnabled { enabled } => {
                Self::SetSliceModeEnabled { enabled }
            }
            compat::UiAction::SetVolume { value_milli } => Self::SetVolume { value_milli },
            compat::UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            compat::UiAction::SeekWaveformPrecise { position_nanos } => {
                Self::SeekWaveformPrecise { position_nanos }
            }
            compat::UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise { position_nanos }
            }
            compat::UiAction::SeekWaveform { position_milli } => {
                Self::SeekWaveform { position_milli }
            }
            compat::UiAction::SetWaveformCursor { position_milli } => {
                Self::SetWaveformCursor { position_milli }
            }
            compat::UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt { anchor_micros }
            }
            compat::UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise { anchor_nanos }
            }
            compat::UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide { anchor_micros }
            }
            compat::UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide { position_micros }
            }
            compat::UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            compat::UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            },
            compat::UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            },
            compat::UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
            compat::UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
            compat::UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve { curve_milli }
            }
            compat::UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve { curve_milli }
            }
            compat::UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            compat::UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
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
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                over_browser_list,
                shift_down,
                alt_down,
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
                pointer_micros,
                start_micros,
                end_micros,
            },
            compat::UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            compat::UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            compat::UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
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
                center_micros,
                center_nanos,
            },
            compat::UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
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
            UiAction::SelectColumn { index } => Self::SelectColumn { index },
            UiAction::MoveColumn { delta } => Self::MoveColumn { delta },
            UiAction::ToggleTransport => Self::ToggleTransport,
            UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            UiAction::PlayFromStart => Self::PlayFromStart,
            UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            UiAction::PlayWaveformAtPrecise { position_nanos } => {
                Self::PlayWaveformAtPrecise { position_nanos }
            }
            UiAction::HandleEscape => Self::HandleEscape,
            UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            UiAction::FocusFolderPanel => Self::FocusFolderPanel,
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
            UiAction::FocusFolderSearch => Self::FocusFolderSearch,
            UiAction::SetFolderSearch { query } => Self::SetFolderSearch { query },
            UiAction::ToggleShowAllFolders => Self::ToggleShowAllFolders,
            UiAction::ToggleFolderFlattenedView => Self::ToggleFolderFlattenedView,
            UiAction::FocusSourceRow { index } => Self::FocusSourceRow { index },
            UiAction::SelectSourceRow { index } => Self::SelectSourceRow { index },
            UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta },
            UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            UiAction::ReloadSourceRow { index } => Self::ReloadSourceRow { index },
            UiAction::HardSyncSourceRow { index } => Self::HardSyncSourceRow { index },
            UiAction::OpenSourceFolderRow { index } => Self::OpenSourceFolderRow { index },
            UiAction::RemoveSourceRow { index } => Self::RemoveSourceRow { index },
            UiAction::FocusFolderRow { index } => Self::FocusFolderRow { index },
            UiAction::ActivateFolderRow { index } => Self::ActivateFolderRow { index },
            UiAction::ToggleFolderRowExpanded { index } => Self::ToggleFolderRowExpanded { index },
            UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta },
            UiAction::StartNewFolder => Self::StartNewFolder,
            UiAction::StartNewFolderAtFolderRow { index } => {
                Self::StartNewFolderAtFolderRow { index }
            }
            UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            UiAction::SetFolderCreateInput { value } => Self::SetFolderCreateInput { value },
            UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            UiAction::StartFolderRename => Self::StartFolderRename,
            UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta },
            UiAction::SetBrowserViewStart { visible_row } => {
                Self::SetBrowserViewStart { visible_row }
            }
            UiAction::FocusBrowserRow { visible_row } => Self::FocusBrowserRow { visible_row },
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
                Self::ToggleBrowserRowSelection { visible_row }
            }
            UiAction::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartContentItemDrag {
                visible_row,
                pointer_x,
                pointer_y,
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
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            },
            UiAction::FinishBrowserSampleDrag => Self::FinishContentItemDrag,
            UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow { visible_row }
            }
            UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeBrowserSelection { visible_row }
            }
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta }
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta }
            }
            UiAction::ToggleFocusedBrowserRowSelection => Self::ToggleFocusedBrowserRowSelection,
            UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query },
            UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter { level, invert }
            }
            UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter { bucket, invert }
            }
            UiAction::ToggleBrowserSampleMark => Self::ToggleContentMark,
            UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            UiAction::ToggleBrowserTagNamedFilter { invert } => {
                Self::ToggleBrowserDerivedLabelFilter { invert }
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
                Self::ToggleBrowserDuplicateCleanupKeep { visible_row }
            }
            UiAction::ConfirmBrowserDuplicateCleanup => Self::ConfirmBrowserDuplicateCleanup,
            UiAction::PlayRandomSample => Self::PlayRandomContentItem,
            UiAction::PlayPreviousRandomSample => Self::PlayPreviousRandomContentItem,
            UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta }
            }
            UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map },
            UiAction::FocusBrowserTagSidebarInput => Self::FocusBrowserPillEditorInput,
            UiAction::SetBrowserTagSidebarInput { value } => {
                Self::SetBrowserPillEditorInput { value }
            }
            UiAction::CommitBrowserTagSidebarInput => Self::CommitBrowserPillEditorInput,
            UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped }
            }
            UiAction::ToggleBrowserSidebarNormalTag { label } => {
                Self::ToggleBrowserPillOption { label }
            }
            UiAction::FocusMapSample { sample_id } => Self::FocusSpatialContentItem {
                content_id: sample_id,
            },
            UiAction::SetPromptInput { value } => Self::SetPromptInput { value },
            UiAction::StartBrowserRename => Self::StartBrowserRename,
            UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection { visible_row }
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
                Self::ToggleWaveformSliceSelection { index }
            }
            UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index }
            }
            UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index }
            }
            UiAction::MoveWaveformSliceFocus { delta } => Self::MoveWaveformSliceFocus { delta },
            UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            UiAction::DeleteLoadedWaveformSample => Self::DeleteLoadedWaveformContent,
            UiAction::SlideWaveformSelection { delta, fine } => {
                Self::SlideWaveformSelection { delta, fine }
            }
            UiAction::ConfirmPrompt => Self::ConfirmPrompt,
            UiAction::CancelPrompt => Self::CancelPrompt,
            UiAction::CancelProgress => Self::CancelProgress,
            UiAction::CopySelectionToClipboard => Self::CopySelectionToClipboard,
            UiAction::ToggleHotkeyOverlay => Self::ToggleHotkeyOverlay,
            UiAction::CopyStatusLog => Self::CopyStatusLog,
            UiAction::OpenFeedbackIssuePrompt => Self::OpenFeedbackIssuePrompt,
            UiAction::MoveTrashedSamplesToFolder => Self::MoveDiscardedItemsToFolder,
            UiAction::SetInputMonitoringEnabled { enabled } => {
                Self::SetInputMonitoringEnabled { enabled }
            }
            UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled }
            }
            UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled }
            }
            UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled }
            }
            UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            UiAction::SetWaveformChannelView { stereo } => Self::SetWaveformChannelView { stereo },
            UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled }
            }
            UiAction::SetBpmSnapEnabled { enabled } => Self::SetBpmSnapEnabled { enabled },
            UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled }
            }
            UiAction::AdjustWaveformBpm { delta } => Self::AdjustWaveformBpm { delta },
            UiAction::SetWaveformBpmValue { value_tenths } => {
                Self::SetWaveformBpmValue { value_tenths }
            }
            UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled }
            }
            UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled }
            }
            UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            UiAction::SetSliceModeEnabled { enabled } => Self::SetSliceModeEnabled { enabled },
            UiAction::SetVolume { value_milli } => Self::SetVolume { value_milli },
            UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            UiAction::SeekWaveformPrecise { position_nanos } => {
                Self::SeekWaveformPrecise { position_nanos }
            }
            UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise { position_nanos }
            }
            UiAction::SeekWaveform { position_milli } => Self::SeekWaveform { position_milli },
            UiAction::SetWaveformCursor { position_milli } => {
                Self::SetWaveformCursor { position_milli }
            }
            UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt { anchor_micros }
            }
            UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise { anchor_nanos }
            }
            UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide { anchor_micros }
            }
            UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide { position_micros }
            }
            UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            },
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            },
            UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
            UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
            UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
            UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            },
            UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd { position_micros }
            }
            UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart { position_micros }
            }
            UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve { curve_milli }
            }
            UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart { position_micros }
            }
            UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd { position_micros }
            }
            UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve { curve_milli }
            }
            UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
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
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                over_browser_list,
                shift_down,
                alt_down,
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
                pointer_micros,
                start_micros,
                end_micros,
            },
            UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            UiAction::FinishWaveformEditSelectionDrag => Self::FinishWaveformEditSelectionDrag,
            UiAction::ClearWaveformSelection => Self::ClearWaveformSelection,
            UiAction::ClearWaveformEditSelection => Self::ClearWaveformEditSelection,
            UiAction::ClearWaveformSelections => Self::ClearWaveformSelections,
            UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => Self::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            },
            UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
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

impl From<&SourcesPanelModel> for compat::SourcesPanelModel {
    fn from(value: &SourcesPanelModel) -> Self {
        value.clone()
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
            tag_sidebar: value.pill_editor,
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
            pill_editor: value.tag_sidebar,
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
            chip_state: value.status_state,
            chip_label: value.status_label,
            detail_label: value.detail_label,
            output_host: value.primary_group,
            output_device: value.primary_item,
            output_sample_rate: value.primary_number,
            input_host: value.secondary_group,
            input_device: value.secondary_item,
            input_sample_rate: value.secondary_number,
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
            status_state: value.chip_state,
            status_label: value.chip_label,
            detail_label: value.detail_label,
            primary_group: value.output_host,
            primary_item: value.output_device,
            primary_number: value.output_sample_rate,
            secondary_group: value.input_host,
            secondary_item: value.input_device,
            secondary_number: value.input_sample_rate,
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

impl From<&OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: &OptionsPanelModel) -> Self {
        value.clone()
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

impl From<&WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: &WaveformPanelModel) -> Self {
        value.clone()
    }
}

impl From<&WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: &WaveformChromeModel) -> Self {
        value.clone()
    }
}

impl From<compat::AppModel> for AppModel {
    fn from(value: compat::AppModel) -> Self {
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status,
            audio_engine: value.paired_device.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel,
            progress_overlay: value.progress_overlay,
            confirm_prompt: confirm_prompt_from_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay,
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources,
            browser: value.browser.into(),
            browser_chrome: value.browser_chrome.into(),
            map: value.map,
            waveform: value.waveform,
            waveform_chrome: value.waveform_chrome,
            update: value.update,
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
            status: value.status,
            paired_device: value.audio_engine.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel,
            progress_overlay: value.progress_overlay,
            confirm_prompt: confirm_prompt_to_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay,
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources,
            browser: value.browser.into(),
            browser_chrome: value.browser_chrome.into(),
            map: value.map,
            waveform: value.waveform,
            waveform_chrome: value.waveform_chrome,
            update: value.update,
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
        options_panel: value.options_panel.clone(),
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
        waveform: value.waveform.clone(),
        waveform_chrome: value.waveform_chrome.clone(),
        update: value.update.clone(),
        focus_context: value.focus_context.into(),
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

impl From<compat::NativeMotionModel> for NativeMotionModel {
    fn from(value: compat::NativeMotionModel) -> Self {
        Self {
            transport_running: value.transport_running,
            map_active: value.map_active,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            waveform_selection_milli: value.waveform_selection_milli,
            waveform_slices: value.waveform_slices.into_iter().collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli,
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
            waveform_channel_view: value.waveform_channel_view,
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
            waveform_selection_milli: value.waveform_selection_milli,
            waveform_slices: value.waveform_slices.into_iter().collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli,
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
            waveform_channel_view: value.waveform_channel_view,
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
    let report = native_vello::run_native_shell_vello_app_with_artifacts(
        options,
        CompatNativeAppBridge::new(bridge),
    );
    NativeRunReport {
        artifacts: NativeRuntimeArtifacts {
            startup_timing: report.artifacts.startup_timing,
            shutdown_timing: report.artifacts.shutdown_timing,
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
    local_automation_snapshot_from_native_shell(
        shell_state.automation_snapshot(&layout, &local_model),
    )
}

fn local_automation_snapshot_from_native_shell(
    value: GuiAutomationSnapshot,
) -> NativeGuiAutomationSnapshot {
    NativeGuiAutomationSnapshot {
        schema_version: value.schema_version,
        viewport_width: value.viewport_width,
        viewport_height: value.viewport_height,
        root: local_automation_node_from_native_shell(value.root),
    }
}

fn local_automation_node_from_native_shell(
    value: AutomationNodeSnapshot,
) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot {
        id: automation_node_id_from_generic(value.id),
        role: value.role,
        label: value.label,
        bounds: value.bounds,
        value: value.value,
        enabled: value.enabled,
        selected: value.selected,
        available_actions: value
            .available_actions
            .into_iter()
            .map(automation_action_id_from_generic)
            .collect(),
        metadata: automation_metadata_from_generic(value.metadata),
        children: value
            .children
            .into_iter()
            .map(local_automation_node_from_native_shell)
            .collect(),
    }
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
    use crate::gui::repaint::RepaintSignal;
    use radiant::widgets::{CanvasMessage, WidgetInput, WidgetOutput};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::{fs, path::Path};

    #[test]
    fn native_run_options_map_field_for_field_to_radiant_runtime_options() {
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
    fn sempal_runtime_glue_stays_local_until_generic_runtime_cutover() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let adapter =
            fs::read_to_string(manifest_dir.join("src/gui_runtime/native_shell_runtime.rs"))
                .expect("native shell runtime adapter");
        let runtime = fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello.rs"))
            .expect("local native vello runtime");
        let runtime_prelude = fs::read_to_string(
            manifest_dir.join("src/gui_runtime/native_vello/legacy_shell_prelude.rs"),
        )
        .expect("local native vello prelude");
        let public_runtime =
            fs::read_to_string(manifest_dir.join("src/gui_runtime/mod.rs")).expect("runtime mod");

        assert!(
            adapter.contains("crate::compat_app_contract as compat")
                && adapter.contains("native_vello::run_native_shell_vello_app_with_artifacts")
                && adapter.contains("local_automation_snapshot_from_native_shell")
                && adapter
                    .contains("crate::compat_app_contract::capture_native_shell_shot_snapshot"),
            "Sempal compatibility conversion, runtime launch, automation, and shot snapshots should stay in the local runtime adapter"
        );
        assert!(
            !adapter.contains(&format!("{}{}", "radiant::compat::", "legacy_shell"))
                && !adapter.contains(&format!(
                    "{}{}",
                    "run_legacy_native_vello_", "app_with_artifacts"
                )),
            "OPT-275 must not route Sempal runtime glue through Radiant's legacy-shell facade"
        );
        assert!(
            runtime_prelude.contains("crate::compat_app_contract")
                && runtime.contains("pub(super) use legacy_shell_runtime::run_legacy_shell_vello_app_with_artifacts as run_native_shell_vello_app_with_artifacts"),
            "the transitional native-Vello runner should be Sempal-owned until OPT-277 switches to RuntimeBridge"
        );
        assert!(
            public_runtime.contains("Sempal GUI runtime host integration")
                && public_runtime.contains("Product shell composition, automation snapshots")
                && public_runtime.contains("Launching Sempal native Vello runtime"),
            "runtime boundary docs and logs should describe Sempal-owned compatibility glue, not a Radiant legacy runtime"
        );
    }

    #[test]
    fn sempal_generic_runtime_bridge_routes_messages_repaint_exit_and_snapshots() {
        let repaint_installed = Arc::new(AtomicBool::new(false));
        let mut bridge = SempalRuntimeBridge::new(RecordingBridge {
            model: Arc::new(NativeAppModel::default()),
            reduced: Vec::new(),
            repaint_installed: Arc::clone(&repaint_installed),
            exit_status: Some(String::from("clean")),
        });

        let surface = bridge.project_surface();
        let action = surface
            .dispatch_widget_output(
                1,
                WidgetOutput::Canvas(CanvasMessage::Input {
                    input: WidgetInput::PointerPress {
                        position: radiant::gui::types::Point::new(4.0, 5.0),
                        button: radiant::widgets::PointerButton::Primary,
                    },
                }),
            )
            .expect("generic canvas should map input into a Sempal action");
        bridge.reduce_message(action);
        assert_eq!(bridge.inner.reduced, vec![UiAction::HandleEscape]);

        bridge.install_repaint_signal(Arc::new(TestRepaintSignal));
        assert!(repaint_installed.load(Ordering::Acquire));

        let exit = bridge.on_runtime_exit().expect("shutdown artifact");
        assert_eq!(exit["status"], "clean");

        let snapshot = bridge.capture_gui_automation_snapshot([1280.0, 720.0]);
        assert_eq!(snapshot.root.id.0, "shell.root");

        let shortcut = bridge.resolve_key_press(
            None,
            RadiantKeyPress {
                key: radiant::gui::input::KeyCode::G,
                command: false,
                shift: false,
                alt: false,
            },
            RadiantFocusSurface::None,
        );
        assert!(shortcut.handled);
        assert_eq!(
            shortcut.pending_chord,
            Some(RadiantKeyPress {
                key: radiant::gui::input::KeyCode::G,
                command: false,
                shift: false,
                alt: false,
            })
        );
    }

    #[test]
    fn sempal_root_dependency_no_longer_enables_radiant_legacy_shell() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cargo = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("root manifest");
        let adapter =
            fs::read_to_string(manifest_dir.join("src/gui_runtime/native_shell_runtime.rs"))
                .expect("native shell runtime adapter");

        assert!(
            cargo.contains("radiant = { path = \"vendor/radiant\" }")
                && !cargo.contains("features = [\"legacy-shell\"]"),
            "Sempal should consume Radiant without the legacy-shell feature after OPT-277"
        );
        assert!(
            adapter.contains(
                "impl<B: NativeAppBridge> RuntimeBridge<UiAction> for SempalRuntimeBridge<B>"
            ) && adapter.contains("fn resolve_key_press(")
                && adapter.contains("fn install_repaint_signal(")
                && adapter.contains("fn on_runtime_exit("),
            "Sempal should own a generic Radiant RuntimeBridge adapter for shortcut, repaint, and exit routing"
        );
    }

    struct RecordingBridge {
        model: Arc<NativeAppModel>,
        reduced: Vec<UiAction>,
        repaint_installed: Arc<AtomicBool>,
        exit_status: Option<String>,
    }

    impl NativeAppBridge for RecordingBridge {
        fn project_model(&mut self) -> Arc<NativeAppModel> {
            Arc::clone(&self.model)
        }

        fn reduce_action(&mut self, action: UiAction) {
            self.reduced.push(action);
        }

        fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {
            self.repaint_installed.store(true, Ordering::Release);
        }

        fn on_runtime_exit(&mut self) -> Option<super::super::NativeShutdownTimingArtifact> {
            Some(super::super::NativeShutdownTimingArtifact {
                status: self.exit_status.take()?,
                failure_reason: None,
                bridge_exit_flush_ms: None,
                config_persist_ms: None,
                controller_jobs_shutdown_ms: None,
                analysis_shutdown_ms: None,
                controller_shutdown_ms: None,
                runtime_exit_total_ms: None,
            })
        }
    }

    struct TestRepaintSignal;

    impl RepaintSignal for TestRepaintSignal {
        fn request_repaint(&self) {}
    }
}
