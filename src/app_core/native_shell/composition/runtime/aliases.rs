pub use crate::gui::automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot,
};
pub use crate::gui::chrome::ContentViewChrome as BrowserChromeModel;
pub use crate::gui::focus::FocusSurface as FocusContextModel;
pub use crate::gui::input::KeyPress;
pub use crate::gui::list::ContentListActions as BrowserActionsModel;
pub use crate::gui::shortcuts::ShortcutResolution;
pub use crate::app_core::actions::{
    NativeBrowserRowModel as BrowserRowModel,
    NativeBrowserRowProcessingState as BrowserRowProcessingState,
    NativeBrowserTagPillModel as BrowserPillModel,
    NativeBrowserTagSidebarModel as BrowserPillEditorModel,
    NativeBrowserTagState as BrowserPillState,
    NativeColumnModel as ColumnModel,
    NativeFolderActionsModel as FolderActionsModel,
    NativeFolderPaneIdModel as FolderPaneIdModel,
    NativeFolderPaneModel as FolderPaneModel,
    NativeFolderRecoveryModel as FolderRecoveryModel,
    NativeFolderRowKind as FolderRowKind,
    NativeFolderRowModel as FolderRowModel,
    NativeFrameBuildResult as FrameBuildResult,
    NativeMapPanelModel as MapPanelModel,
    NativeMapPointModel as MapPointModel,
    NativeMapRenderModeModel as MapRenderModeModel,
    NativePlaybackAgeBucket as PlaybackAgeBucket,
    NativePlaybackAgeFilterChip as PlaybackAgeFilterChip,
    NativeRetainedVec as RetainedVec,
    NativeSourceRowModel as SourceRowModel,
};

use super::UiAction;

/// Compatibility alias for the generic shortcut resolution DTO.
pub type HotkeyResolution = ShortcutResolution<UiAction>;

/// Summary of browser/list state consumed by the native shell.
pub type BrowserPanelModel =
    crate::gui::list::ContentListPanel<BrowserRowModel, BrowserPillEditorModel>;
