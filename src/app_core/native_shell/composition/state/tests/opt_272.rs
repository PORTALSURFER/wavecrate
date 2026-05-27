use super::*;
use crate::app_core::native_shell::runtime_contract::{
    FolderPaneIdModel, SourceRowModel, folder_row_model,
};
use crate::gui::types::Vector2;

fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
    let mut model = AppModel::default();
    for visible_row in 0..total {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
    model.browser.autoscroll = true;
    model
}

fn folder_model_with_rows(total_rows: usize, focused_row: usize) -> AppModel {
    let mut model = AppModel::default();
    model.sources.rows.push(SourceRowModel::new(
        String::from("source"),
        String::from("detail"),
        true,
        false,
    ));
    model.sources.upper_folder_pane.active = true;
    model.sources.upper_folder_pane.has_item = true;
    model.sources.upper_folder_pane.focused_tree_row = Some(focused_row);
    model.sources.active_folder_pane = FolderPaneIdModel::Upper;
    for row_index in 0..total_rows {
        model
            .sources
            .upper_folder_pane
            .tree_rows
            .push(folder_row_model(
                format!("folder_{row_index:03}"),
                String::new(),
                row_index % 3,
                false,
                row_index == focused_row,
                row_index == 0,
                row_index + 1 < total_rows,
                true,
            ));
    }
    model
}

/// Build a populated single-sidebar fixture for source/folder geometry checks.
fn populated_single_sidebar_model() -> AppModel {
    let mut model = folder_model_with_rows(48, 4);
    model.sources.rows.clear();
    for index in 0..12 {
        model.sources.rows.push(SourceRowModel::new(
            format!("source_{index:02}"),
            format!("detail_{index:02}"),
            index == 4,
            false,
        ));
    }
    model
}

#[path = "opt_272/filters.rs"]
mod filters;
#[path = "opt_272/scrollbars.rs"]
mod scrollbars;
#[path = "opt_272/sidebar_workspace.rs"]
mod sidebar_workspace;
#[path = "opt_272/tag_library.rs"]
mod tag_library;
#[path = "opt_272/toolbar_options.rs"]
mod toolbar_options;
