//! Sempal-owned compatibility contract for the current native shell.
//!
//! These types are the app-specific bridge surface used by Sempal while the
//! native shell migrates onto generic Radiant runtime APIs. Generic widget,
//! layout, input, rendering, and invalidation primitives still come from
//! `radiant`; Sempal-specific shell models and actions live here.
#![allow(dead_code, unused_imports)]

#[path = "app_core/native_shell/composition/runtime/actions/mod.rs"]
mod actions;
#[path = "app_core/native_shell/composition/runtime/aliases.rs"]
mod aliases;
#[path = "app_core/native_shell/composition/runtime/bridge.rs"]
mod bridge;
#[path = "app_core/native_shell/composition/runtime/motion.rs"]
mod motion;
#[path = "app_core/native_shell/composition/runtime/shell.rs"]
mod shell;
#[path = "app_core/native_shell/composition/runtime/shell_snapshot.rs"]
mod shell_snapshot;

pub use actions::{BrowserTriageTarget, UiAction};
pub use aliases::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, BrowserPillEditorModel,
    BrowserPillModel, BrowserPillState, BrowserRowModel, BrowserRowProcessingState, ColumnModel,
    FocusContextModel, FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel,
    FolderRowKind, FolderRowModel, FrameBuildResult, GuiAutomationSnapshot, HotkeyResolution,
    KeyPress, MapPanelModel, MapPointModel, MapRenderModeModel, PlaybackAgeBucket,
    PlaybackAgeFilterChip, RetainedVec, ShortcutResolution, SourceRowModel,
};
pub use bridge::NativeAppBridge;
pub use crate::app_core::actions::{
    NativeDirtySegments as DirtySegments, NativeNormalizedRangeModel as NormalizedRangeModel,
    NativeOptionsPanelModel as OptionsPanelModel, NativeSegmentRevisions as SegmentRevisions,
    NativeSourcesPanelModel as SourcesPanelModel, NativeWaveformChromeModel as WaveformChromeModel,
    NativeWaveformPanelModel as WaveformPanelModel,
};
pub use motion::NativeMotionModel;
pub use shell::{
    AppModel, ConfirmPromptKind, ConfirmPromptModel, DragOverlayModel, PairedDevicePanelModel,
    PairedPickerOptionModel, PairedPickerTargetModel, PairedPickerValueModel, ProgressOverlayModel,
    StatusBarModel, StatusChipStateModel, SummaryFieldModel, UpdatePanelModel, UpdateStatusModel,
};
pub use shell_snapshot::{NativeShellShotSnapshot, capture_native_shell_shot_snapshot};
