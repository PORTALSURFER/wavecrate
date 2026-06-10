use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::FolderTreeViewModel;
use crate::native_app::sample_library::folder_browser::{
    FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS, FolderBrowserMessage, TREE_DEPTH_INDENT,
    TREE_ROW_HEIGHT, VisibleFolder,
};

use super::tree_hit_target::FolderTreeHitTarget;

const FOLDER_EXPANDER_WIDTH: f32 = 28.0;
const FOLDER_TREE_GUIDE_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 126,
    b: 64,
    a: 152,
};

pub(super) fn folder_tree_section(model: FolderTreeViewModel) -> ui::View<GuiMessage> {
    ui::column([
        folder_tree_view(model.visible_folders, model.window, model.drag_revision),
        selected_folder_status(model.selected_folder_status_label),
    ])
    .spacing(0.0)
    .fill_width()
    .fill_height()
}

fn folder_tree_view(
    visible_folders: Vec<VisibleFolder>,
    window: ui::VirtualListWindow,
    drag_revision: u64,
) -> ui::View<GuiMessage> {
    folder_tree_window(visible_folders, window, drag_revision)
        .id(FOLDER_TREE_LIST_ID)
        .fill_width()
        .fill_height()
}

fn folder_tree_window(
    visible_folders: Vec<VisibleFolder>,
    window: ui::VirtualListWindow,
    drag_revision: u64,
) -> ui::View<GuiMessage> {
    ui::virtual_tree_list_window(
        window,
        TREE_ROW_HEIGHT,
        &folder_tree_guide_rows(&visible_folders),
        folder_tree_guide_style(),
        |index| folder_row(visible_folders[index].clone(), drag_revision),
        TREE_ROW_HEIGHT * FOLDER_TREE_OVERSCAN_ROWS as f32,
    )
    .on_scroll_update({
        move |update| {
            GuiMessage::FolderTreeWindowChanged(ui::virtual_list_window_change_for_scroll(
                update,
                TREE_ROW_HEIGHT,
                window,
                FOLDER_TREE_OVERSCAN_ROWS,
            ))
        }
    })
    .style(ui::WidgetStyle::default())
    .fill_height()
}

fn folder_row(folder: VisibleFolder, drag_revision: u64) -> ui::View<GuiMessage> {
    let id = folder.id.clone();
    if let (Some(draft), Some(input_id)) = (folder.rename_draft.clone(), folder.rename_input_id) {
        let caret = draft.chars().count();
        return ui::row([
            ui::tree_guide_indent(folder.depth, folder_tree_guide_style()),
            ui::text_input(draft)
                .selection(0, caret)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
                })
                .id(input_id)
                .key(format!("folder-rename-input-{id}"))
                .fill_width()
                .height(22.0),
        ])
        .key(format!("folder-row-{id}"))
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
        .fill_width()
        .height(TREE_ROW_HEIGHT)
        .spacing(1.0)
        .hoverable();
    }

    let label_text = folder.name.clone();
    let expander = if folder.has_children && !folder.is_source_root {
        let expander_label = if folder.expanded { "[-]" } else { "[+]" };
        ui::button(expander_label)
            .subtle()
            .message(GuiMessage::FolderBrowser(
                FolderBrowserMessage::ToggleFolderExpansion(id.clone()),
            ))
            .key(format!("folder-expander-{id}"))
            .size(FOLDER_EXPANDER_WIDTH, 22.0)
    } else {
        ui::spacer()
            .key(format!("folder-expander-spacer-{id}"))
            .size(FOLDER_EXPANDER_WIDTH, 22.0)
    };
    let hit_target = ui::custom_widget_direct(FolderTreeHitTarget::new(
        id.clone(),
        label_text,
        folder.selected,
        folder.drop_target,
        folder.drag_active,
        folder.drag_source,
        folder.drop_candidate,
        folder.drop_target_active,
    ))
    .key(format!("folder-row-hit-{id}-{drag_revision}"))
    .fill_width()
    .height(22.0);

    ui::row([
        ui::tree_guide_indent(folder.depth, folder_tree_guide_style()),
        expander,
        hit_target.fill_width().height(22.0),
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.selected || folder.drop_target {
        ui::WidgetStyle::subtle(ui::WidgetTone::Accent)
    } else {
        ui::WidgetStyle::default()
    })
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
}

fn folder_tree_guide_style() -> ui::TreeGuideStyle {
    ui::TreeGuideStyle::new(TREE_DEPTH_INDENT, TREE_ROW_HEIGHT, FOLDER_TREE_GUIDE_COLOR)
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

fn selected_folder_status(label: String) -> ui::View<GuiMessage> {
    ui::text_line(label, 20.0)
}
