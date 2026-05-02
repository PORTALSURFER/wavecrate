use super::{
    FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel, FolderRowModel,
    RetainedVec, SourceRowModel,
};

/// Sidebar model for source browsing controls.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourcesPanelModel {
    /// Header text for the source panel.
    pub header: String,
    /// Active source-search query.
    pub search_query: String,
    /// Pane that currently drives browser and waveform state.
    pub active_folder_pane: FolderPaneIdModel,
    /// Upper fixed folder pane.
    pub upper_folder_pane: FolderPaneModel,
    /// Lower fixed folder pane.
    pub lower_folder_pane: FolderPaneModel,
    /// Active folder-search query.
    pub tree_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_items: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_items: bool,
    /// Whether folder filtering includes descendant files in a flattened list.
    pub flattened_view: bool,
    /// Whether the folder flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
    /// Selected row index, if any.
    pub selected_row: Option<usize>,
    /// Source row currently hydrating in the background, if any.
    pub loading_row: Option<usize>,
    /// Source row currently running a background file or folder mutation, if any.
    pub mutation_busy_row: Option<usize>,
    /// Focused folder row index, if any.
    pub focused_tree_row: Option<usize>,
    /// Rows to render in the source panel.
    pub rows: RetainedVec<SourceRowModel>,
    /// Folder rows to render in the folder browser section.
    pub tree_rows: RetainedVec<FolderRowModel>,
    /// Folder action availability for native sidebar controls.
    pub tree_actions: FolderActionsModel,
    /// Folder delete-recovery summary for native sidebar status.
    pub recovery: FolderRecoveryModel,
}

impl SourcesPanelModel {
    /// Borrow one pane model by id.
    pub fn folder_pane(&self, pane: FolderPaneIdModel) -> &FolderPaneModel {
        pane.select(&self.upper_folder_pane, &self.lower_folder_pane)
    }

    /// Borrow the pane that currently drives browser and waveform state.
    pub fn active_folder_pane_model(&self) -> &FolderPaneModel {
        self.folder_pane(self.active_folder_pane)
    }

    /// Return this source/sidebar model as a generic split-pane sidebar state.
    pub fn split_pane_sidebar(
        &self,
    ) -> crate::gui::panel::SplitPaneSidebarState<SourceRowModel, FolderRowModel> {
        crate::gui::panel::SplitPaneSidebarState {
            header: self.header.clone(),
            search_query: self.search_query.clone(),
            active_pane: self.active_folder_pane,
            upper_pane: self.upper_folder_pane.clone(),
            lower_pane: self.lower_folder_pane.clone(),
            tree_search_query: self.tree_search_query.clone(),
            show_all_items: self.show_all_items,
            can_toggle_show_all_items: self.can_toggle_show_all_items,
            flattened_view: self.flattened_view,
            can_toggle_flattened_view: self.can_toggle_flattened_view,
            selected_row: self.selected_row,
            loading_row: self.loading_row,
            mutation_busy_row: self.mutation_busy_row,
            focused_tree_row: self.focused_tree_row,
            rows: self.rows.clone(),
            tree_rows: self.tree_rows.clone(),
            tree_actions: self.tree_actions.clone(),
            recovery: self.recovery.clone(),
        }
    }
}
