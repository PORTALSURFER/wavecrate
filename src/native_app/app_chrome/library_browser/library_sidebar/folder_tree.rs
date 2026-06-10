use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::{
    SIDEBAR_ROW_HOVER_FILL, SIDEBAR_ROW_PRESSED_FILL,
};
use crate::native_app::app_chrome::view_models::library_sidebar::FolderTreeViewModel;
use crate::native_app::sample_library::folder_browser::{
    FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS, FolderBrowserMessage, TREE_DEPTH_INDENT,
    TREE_ROW_HEIGHT, VisibleFolder,
};

const FOLDER_EXPANDER_WIDTH: f32 = 28.0;
const FOLDER_TREE_GUIDE_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 126,
    b: 64,
    a: 152,
};
const FOLDER_TREE_SELECTED_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 34,
};
const FOLDER_TREE_ACTIVE_TARGET_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 130,
    b: 78,
    a: 220,
};
const FOLDER_TREE_CANDIDATE_HOVER_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 122,
    b: 74,
    a: 150,
};
const FOLDER_TREE_DROP_OUTLINE: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 180,
    b: 130,
    a: 235,
};
const FOLDER_TREE_HIGHLIGHTED_LABEL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 238,
    b: 224,
    a: 255,
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
        |index| folder_row(&visible_folders[index], drag_revision),
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

fn folder_row(folder: &VisibleFolder, drag_revision: u64) -> ui::View<GuiMessage> {
    let id = folder.id.clone();
    if let (Some(draft), Some(input_id)) = (&folder.rename_draft, folder.rename_input_id) {
        let caret = draft.chars().count();
        return ui::row([
            ui::tree_guide_indent(folder.depth, folder_tree_guide_style()),
            ui::text_input(draft.clone())
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

    ui::tree_row(folder.name.clone())
        .depth(folder.depth)
        .expanded(folder.expanded)
        .has_children(folder.has_children && !folder.is_source_root)
        .selected(folder.selected)
        .drag_drop_state(folder_tree_drag_drop_state(folder))
        .row_height(TREE_ROW_HEIGHT)
        .expander_width(FOLDER_EXPANDER_WIDTH)
        .guide_style(folder_tree_guide_style())
        .palette(folder_tree_palette())
        .drop_target_outline(folder_tree_drop_target_outline())
        .highlighted_label_color(FOLDER_TREE_HIGHLIGHTED_LABEL)
        .row_key(format!("folder-row-{id}"))
        .hit_key(format!("folder-row-hit-{id}-{drag_revision}"))
        .on_toggle({
            let id = id.clone();
            move || {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ToggleFolderExpansion(id.clone()))
            }
        })
        .interactive_actions(folder_row_actions(
            id,
            folder.drop_candidate,
            folder.drop_target_active,
        ))
}

fn folder_row_actions(
    id: String,
    drop_candidate: bool,
    drop_target_active: bool,
) -> ui::InteractiveRowActions<GuiMessage> {
    ui::InteractiveRowActions::new().activate_or_double_secondary_drag_drop_target_key(
        id,
        |id| GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id)),
        |id, position| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::OpenFolderContextMenu(id, position))
        },
        |id, drag| GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(id, drag)),
        |id| GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnFolder(id)),
        move |id, position| {
            GuiMessage::FolderBrowser(folder_hover_drop_message(
                id,
                position,
                drop_candidate,
                drop_target_active,
            ))
        },
    )
}

fn folder_hover_drop_message(
    id: String,
    position: ui::Point,
    drop_candidate: bool,
    drop_target_active: bool,
) -> FolderBrowserMessage {
    if drop_target_active && !drop_candidate {
        FolderBrowserMessage::ClearDropTargetUnless(id, position)
    } else {
        FolderBrowserMessage::HoverDropTarget(id, position)
    }
}

fn folder_tree_drag_drop_state(folder: &VisibleFolder) -> ui::TreeRowDragDropState {
    ui::TreeRowDragDropState {
        drag_active: folder.drag_active,
        drag_source: folder.drag_source,
        drop_candidate: folder.drop_candidate,
        drop_target: folder.drop_target,
        drop_target_active: folder.drop_target_active,
    }
}

fn folder_tree_palette() -> ui::DenseRowPalette {
    ui::DenseRowPalette::new()
        .selected(FOLDER_TREE_SELECTED_FILL)
        .interaction_fills(SIDEBAR_ROW_HOVER_FILL, SIDEBAR_ROW_PRESSED_FILL)
        .active_target(FOLDER_TREE_ACTIVE_TARGET_FILL)
        .candidate_hovered(FOLDER_TREE_CANDIDATE_HOVER_FILL)
}

fn folder_tree_drop_target_outline() -> ui::DenseRowOutlineStyle {
    ui::DenseRowOutlineStyle::new(0.5, FOLDER_TREE_DROP_OUTLINE, 1.5)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_tree_uses_shared_grey_sidebar_hover_fill() {
        let palette = folder_tree_palette();

        assert_eq!(palette.hovered, Some(SIDEBAR_ROW_HOVER_FILL));
        assert_eq!(palette.selected, Some(FOLDER_TREE_SELECTED_FILL));
    }
}
