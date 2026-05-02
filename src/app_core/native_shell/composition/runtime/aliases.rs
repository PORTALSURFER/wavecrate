pub use crate::gui::automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot,
};
pub use crate::gui::chrome::ContentViewChrome as BrowserChromeModel;
pub use crate::gui::feedback::RecoverySummary as FolderRecoveryModel;
pub use crate::gui::focus::FocusSurface as FocusContextModel;
pub use crate::gui::frame::FrameBuildResult;
pub use crate::gui::input::KeyPress;
pub use crate::gui::list::ColumnSummary as ColumnModel;
pub use crate::gui::list::ContentListActions as BrowserActionsModel;
pub use crate::gui::list::ContentListRow as BrowserRowModel;
pub use crate::gui::list::EditableRowKind as FolderRowKind;
pub use crate::gui::list::EditableTreeActions as FolderActionsModel;
pub use crate::gui::list::EditableTreeRow as FolderRowModel;
pub use crate::gui::list::RecencyBucket as PlaybackAgeBucket;
pub use crate::gui::list::RecencyFilterChip as PlaybackAgeFilterChip;
pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;
pub use crate::gui::panel::SplitPaneAssignedRow as SourceRowModel;
pub use crate::gui::panel::SplitPaneSlot as FolderPaneIdModel;
pub use crate::gui::retained::RetainedVec;
pub use crate::gui::selection::TriState as BrowserPillState;
pub use crate::gui::shortcuts::ShortcutResolution;
pub use crate::gui::visualization::PointRenderMode as MapRenderModeModel;
pub use crate::gui::visualization::SpatialPanel as MapPanelModel;
pub use crate::gui::visualization::SpatialPoint as MapPointModel;

use super::UiAction;

/// Compatibility alias for the generic shortcut resolution DTO.
pub type HotkeyResolution = ShortcutResolution<UiAction>;

/// One clickable pill projected into the browser metadata sidebar.
pub type BrowserPillModel = crate::gui::badge::SelectablePill<BrowserPillState>;
/// Browser-local metadata sidebar shown beside the content list.
pub type BrowserPillEditorModel = crate::gui::badge::PillEditorPanel<BrowserPillState>;

/// Summary of browser/list state consumed by the native shell.
pub type BrowserPanelModel =
    crate::gui::list::ContentListPanel<BrowserRowModel, BrowserPillEditorModel>;
/// Projected data for one fixed folder pane shown in the sidebar.
pub type FolderPaneModel = crate::gui::panel::SplitPaneTreePanel<FolderRowModel>;
