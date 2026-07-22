use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::FolderTreeViewModel;
use crate::native_app::sample_library::folder_browser::model::VisibleFolder;
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS, TREE_ROW_HEIGHT,
};

mod identity;
mod rows;
mod status;
#[cfg(test)]
mod tests;

use rows::{folder_row, folder_tree_guide_style};
use status::selected_folder_status;

pub(super) fn folder_tree_section(model: FolderTreeViewModel) -> ui::View<GuiMessage> {
    ui::column([
        folder_tree_view(model.visible_folders, model.window),
        selected_folder_status(
            model.selected_folder_status_label,
            model.selected_source_missing,
            model.include_subfolders_available,
            model.include_subfolders,
            model.show_empty_folders,
            model.help_tooltips_enabled,
        ),
    ])
    .spacing(0.0)
    .fill_width()
    .fill_height()
}

fn folder_tree_view(
    visible_folders: Vec<VisibleFolder>,
    window: ui::VirtualListWindow,
) -> ui::View<GuiMessage> {
    folder_tree_window(visible_folders, window)
        .id(FOLDER_TREE_LIST_ID)
        .fill_width()
        .fill_height()
}

fn folder_tree_window(
    visible_folders: Vec<VisibleFolder>,
    window: ui::VirtualListWindow,
) -> ui::View<GuiMessage> {
    radiant::application::virtual_tree_list_windowed(
        window,
        TREE_ROW_HEIGHT,
        &folder_tree_guide_rows(&visible_folders),
        folder_tree_guide_style(),
        |index| folder_row(&visible_folders[index]),
    )
    .overscan_px(TREE_ROW_HEIGHT * FOLDER_TREE_OVERSCAN_ROWS as f32)
    .on_window_changed(GuiMessage::FolderTreeWindowChanged)
    .view()
    .without_chrome()
    .fill_height()
}

fn folder_tree_guide_rows(folders: &[VisibleFolder]) -> Vec<ui::TreeGuideRow> {
    folders
        .iter()
        .map(|folder| {
            ui::TreeGuideRow::new(
                folder.depth,
                folder.has_children && folder.expanded && !folder.is_source_root,
            )
        })
        .collect()
}
