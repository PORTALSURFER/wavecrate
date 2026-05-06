pub use crate::app_core::actions::{
    NativeAutomationBounds as AutomationBounds, NativeAutomationNodeId as AutomationNodeId,
    NativeAutomationNodeSnapshot as AutomationNodeSnapshot, NativeAutomationRole as AutomationRole,
    NativeBrowserRowModel as BrowserRowModel,
    NativeBrowserRowProcessingState as BrowserRowProcessingState,
    NativeBrowserTagPillModel as BrowserPillModel,
    NativeBrowserTagSidebarModel as BrowserPillEditorModel,
    NativeBrowserTagState as BrowserPillState, NativeColumnModel as ColumnModel,
    NativeFolderActionsModel as FolderActionsModel, NativeFolderPaneIdModel as FolderPaneIdModel,
    NativeFolderPaneModel as FolderPaneModel, NativeFolderRecoveryModel as FolderRecoveryModel,
    NativeFolderRowKind as FolderRowKind, NativeFolderRowModel as FolderRowModel,
    NativeFrameBuildResult as FrameBuildResult,
    NativeGuiAutomationSnapshot as GuiAutomationSnapshot, NativeMapPanelModel as MapPanelModel,
    NativeMapPointModel as MapPointModel, NativeMapRenderModeModel as MapRenderModeModel,
    NativePlaybackAgeBucket as PlaybackAgeBucket,
    NativePlaybackAgeFilterChip as PlaybackAgeFilterChip, NativeRetainedVec as RetainedVec,
    NativeSourceRowModel as SourceRowModel,
};
pub use crate::gui::focus::FocusSurface as FocusContextModel;
pub use crate::gui::input::KeyPress;
pub use crate::gui::shortcuts::ShortcutResolution;

use super::UiAction;

/// Compatibility alias for the generic shortcut resolution DTO.
pub type HotkeyResolution = ShortcutResolution<UiAction>;

/// Summary of browser/list state consumed by the native shell.
///
/// This compatibility DTO stays Sempal-owned because it carries product
/// workflow state such as rating filters, metadata editing, and cleanup mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserPanelModel {
    pub visible_count: usize,
    pub selected_visible_row: Option<usize>,
    pub autoscroll: bool,
    pub view_start_row: usize,
    pub selected_item_count: usize,
    pub search_query: String,
    pub active_rating_filters: [bool; 8],
    pub active_recency_filters: [bool; 3],
    pub marked_filter_active: bool,
    pub derived_label_filter_active: bool,
    pub derived_label_filter_negated: bool,
    pub search_placeholder: Option<String>,
    pub busy: bool,
    pub data_loading: bool,
    pub metadata_pending: bool,
    pub mutation_pending: bool,
    pub similarity_filtered: bool,
    pub duplicate_cleanup_active: bool,
    pub sort_label: Option<String>,
    pub active_tab_label: Option<String>,
    pub focused_item_label: Option<String>,
    pub pill_editor: BrowserPillEditorModel,
    pub anchor_visible_row: Option<usize>,
    pub rows: RetainedVec<BrowserRowModel>,
}

impl Default for BrowserPanelModel {
    fn default() -> Self {
        Self {
            visible_count: 0,
            selected_visible_row: None,
            autoscroll: false,
            view_start_row: 0,
            selected_item_count: 0,
            search_query: String::new(),
            active_rating_filters: [false; 8],
            active_recency_filters: [false; 3],
            marked_filter_active: false,
            derived_label_filter_active: false,
            derived_label_filter_negated: false,
            search_placeholder: None,
            busy: false,
            data_loading: false,
            metadata_pending: false,
            mutation_pending: false,
            similarity_filtered: false,
            duplicate_cleanup_active: false,
            sort_label: None,
            active_tab_label: None,
            focused_item_label: None,
            pill_editor: BrowserPillEditorModel::default(),
            anchor_visible_row: None,
            rows: RetainedVec::new(),
        }
    }
}

impl BrowserPanelModel {
    /// Return whether the browser's derived-label filter is active.
    pub fn derived_label_filter_active(&self) -> bool {
        self.derived_label_filter_active
    }

    /// Return whether the active derived-label filter is negated.
    pub fn derived_label_filter_negated(&self) -> bool {
        self.derived_label_filter_negated
    }

    /// Return the browser metadata pill-editor projection.
    pub fn pill_editor(&self) -> &BrowserPillEditorModel {
        &self.pill_editor
    }
}

/// Browser chrome copy used by the native shell toolbar and tab strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserChromeModel {
    pub items_tab_label: String,
    pub item_column_label: String,
    pub map_tab_label: String,
    pub pill_editor_label: String,
    pub search_prefix_label: String,
    pub search_placeholder: String,
    pub activity_ready_label: String,
    pub activity_busy_label: String,
    pub sort_prefix_label: String,
    pub sort_order_label: String,
    pub similarity_toggle_label: String,
    pub item_count_label: String,
}

impl Default for BrowserChromeModel {
    fn default() -> Self {
        Self {
            items_tab_label: String::from("Items"),
            item_column_label: String::from("Item"),
            map_tab_label: String::from("Map"),
            pill_editor_label: String::from("Pills"),
            search_prefix_label: String::from("Search"),
            search_placeholder: String::from("Search items (Ctrl+F)"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Filtering"),
            sort_prefix_label: String::from("Sort"),
            sort_order_label: String::from("List order"),
            similarity_toggle_label: String::from("points"),
            item_count_label: String::from("0 items"),
        }
    }
}

/// Browser action availability consumed by the native shell action strip.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserActionsModel {
    pub can_rename: bool,
    pub can_delete: bool,
    pub can_edit_pills: bool,
    pub can_process_focused_item: bool,
    pub can_open_focused_item_flow: bool,
    pub random_navigation_enabled: bool,
    pub duplicate_cleanup_active: bool,
    pub pill_editor_open: bool,
}

impl BrowserActionsModel {
    /// Return whether browser metadata pills can be edited.
    pub fn can_edit_pills(&self) -> bool {
        self.can_edit_pills
    }

    /// Return whether the browser metadata pill editor is currently open.
    pub fn pill_editor_open(&self) -> bool {
        self.pill_editor_open
    }
}
