//! Wavecrate-owned UI projection DTOs.
//!
//! These models describe Wavecrate application state as projected for the current
//! UI projection. The runtime adapter in `gui_runtime` converts these app-core
//! DTOs into the Wavecrate-owned native runtime contract consumed by Radiant.

use radiant::gui::frame;
use radiant::gui::range;
use radiant::gui::retained;

mod app_shell;
mod audio_options;
mod automation;
mod browser;
mod motion;
mod retained_segments;
mod sidebar;
mod waveform;

pub use self::app_shell::{
    AppModel, ColumnModel, ConfirmPromptKind, ConfirmPromptModel, DragOverlayModel,
    FocusContextModel, MapPanelModel, MapPointModel, MapRenderModeModel, ProgressOverlayModel,
    StatusBarModel, UpdatePanelModel, UpdateStatusModel,
};
pub use self::audio_options::{
    AudioEngineChipStateModel, AudioEngineModel, AudioFieldModel, AudioOptionItemModel,
    AudioOptionValueModel, AudioPickerTargetModel, OptionsPanelModel,
};
pub use self::automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot,
};
pub use self::browser::{
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, BrowserRowModel,
    BrowserRowProcessingState, BrowserTagPillModel, BrowserTagSidebarModel, BrowserTagState,
    PlaybackAgeBucket, PlaybackAgeFilterChip,
};
pub use self::motion::NativeMotionModel;
pub use self::retained_segments::{DirtySegments, SegmentRevisions};
pub use self::sidebar::{
    FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel, FolderRowKind,
    FolderRowModel, SourceRowModel, SourcesPanelModel, folder_row_model,
};
pub use self::waveform::{
    WaveformChannelViewModel, WaveformChromeModel, WaveformChromeStateModel,
    WaveformEditPreviewModel, WaveformFeedbackEventsModel, WaveformImagePreviewModel,
    WaveformMotionModel, WaveformPanelModel, WaveformPresentationModel, WaveformSlicePreviewModel,
    WaveformSurfaceModel, WaveformToolStateModel, WaveformTransportModel, WaveformViewportModel,
    parse_waveform_tempo_number_text,
};

/// Shared storage used by retained app-model snapshots.
pub type RetainedVec<T> = retained::RetainedVec<T>;

/// Frame-level feedback from renderer to host bridge.
pub type FrameBuildResult = frame::FrameBuildResult;

/// Normalized interval with deterministic milli, micro, and nano projections.
pub type NormalizedRangeModel = range::NormalizedRange;

#[cfg(test)]
mod projection_tests;
