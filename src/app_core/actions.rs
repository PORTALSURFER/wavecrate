//! Migration-facing aliases for native runtime action/model types.
//!
//! These aliases centralize runtime-facing type dependencies in `app_core`,
//! so bridge/controller glue does not import concrete runtime types directly.

/// Native runtime browser action metadata model.
pub type NativeBrowserActionsModel = radiant::app::BrowserActionsModel;

/// Native runtime browser chrome model.
pub type NativeBrowserChromeModel = radiant::app::BrowserChromeModel;

/// Native runtime browser panel model.
pub type NativeBrowserPanelModel = radiant::app::BrowserPanelModel;

/// Native runtime browser row model.
pub type NativeBrowserRowModel = radiant::app::BrowserRowModel;

/// Native runtime browser tag target used by keyboard and pointer triage actions.
pub type NativeBrowserTagTarget = radiant::app::BrowserTagTarget;

/// Native runtime UI action payload.
pub type NativeUiAction = radiant::app::UiAction;

/// Native runtime projected app model.
pub type NativeAppModel = radiant::app::AppModel;

/// Native runtime frame build result payload.
pub type NativeFrameBuildResult = radiant::app::FrameBuildResult;

/// Native runtime projection-dirty segment mask.
pub type NativeDirtySegments = radiant::app::DirtySegments;

/// Native runtime motion-only model payload.
pub type NativeMotionModel = radiant::app::NativeMotionModel;

/// Native runtime table column summary model.
pub type NativeColumnModel = radiant::app::ColumnModel;

/// Native runtime confirm prompt kind descriptor.
pub type NativeConfirmPromptKind = radiant::app::ConfirmPromptKind;

/// Native runtime confirm prompt model.
pub type NativeConfirmPromptModel = radiant::app::ConfirmPromptModel;

/// Native runtime drag overlay model.
pub type NativeDragOverlayModel = radiant::app::DragOverlayModel;

/// Native runtime folder actions model.
pub type NativeFolderActionsModel = radiant::app::FolderActionsModel;

/// Native runtime folder recovery model.
pub type NativeFolderRecoveryModel = radiant::app::FolderRecoveryModel;

/// Native runtime folder row model.
pub type NativeFolderRowModel = radiant::app::FolderRowModel;

/// Native runtime map panel model.
pub type NativeMapPanelModel = radiant::app::MapPanelModel;

/// Native runtime map point model.
pub type NativeMapPointModel = radiant::app::MapPointModel;

/// Native runtime map render mode model.
pub type NativeMapRenderModeModel = radiant::app::MapRenderModeModel;

/// Native runtime normalized range model.
pub type NativeNormalizedRangeModel = radiant::app::NormalizedRangeModel;

/// Native runtime progress overlay model.
pub type NativeProgressOverlayModel = radiant::app::ProgressOverlayModel;

/// Native runtime source row model.
pub type NativeSourceRowModel = radiant::app::SourceRowModel;

/// Native runtime sources panel model.
pub type NativeSourcesPanelModel = radiant::app::SourcesPanelModel;

/// Native runtime status bar model.
pub type NativeStatusBarModel = radiant::app::StatusBarModel;

/// Native runtime update panel model.
pub type NativeUpdatePanelModel = radiant::app::UpdatePanelModel;

/// Native runtime update status indicator model.
pub type NativeUpdateStatusModel = radiant::app::UpdateStatusModel;

/// Native runtime waveform chrome model.
pub type NativeWaveformChromeModel = radiant::app::WaveformChromeModel;

/// Native runtime waveform panel model.
pub type NativeWaveformPanelModel = radiant::app::WaveformPanelModel;

/// Native runtime bridge trait used by host launchers.
pub use radiant::app::NativeAppBridge;
