use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::SIDEBAR_ROW_STYLE;
use crate::native_app::app_chrome::view_models::library_sidebar::FolderTreeViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::model::VisibleFolder;
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS, TREE_DEPTH_INDENT, TREE_ROW_HEIGHT,
};

#[cfg(test)]
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::sidebar_row_full_palette;

const FOLDER_EXPANDER_WIDTH: f32 = 28.0;
const FOLDER_TREE_HIGHLIGHTED_LABEL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 238,
    b: 224,
    a: 255,
};
const FOLDER_TREE_EMPTY_LABEL: ui::Rgba8 = ui::Rgba8 {
    r: 142,
    g: 148,
    b: 156,
    a: 255,
};
const FOLDER_TREE_SELECTED_HOVER_MARKER_ALPHA: u8 = 245;
const FOLDER_TREE_SELECTED_HOVER_MARKER_WIDTH: f32 = 3.0;
const FOLDER_LOCK_ICON_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 232,
    g: 221,
    b: 190,
    a: 235,
};
const FOLDER_LOCK_INHERITED_ICON_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 158,
    g: 164,
    b: 172,
    a: 220,
};

mod status;
use status::selected_folder_status;

pub(super) fn folder_tree_section(model: FolderTreeViewModel) -> ui::View<GuiMessage> {
    ui::column([
        folder_tree_view(model.visible_folders, model.window),
        selected_folder_status(
            model.selected_folder_status_label,
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
    ui::virtual_tree_list_window(
        window,
        TREE_ROW_HEIGHT,
        &folder_tree_guide_rows(&visible_folders),
        folder_tree_guide_style(),
        |index| folder_row(&visible_folders[index]),
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

fn folder_row(folder: &VisibleFolder) -> ui::View<GuiMessage> {
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

    let row = ui::tree_row(folder.name.clone())
        .depth(folder.depth)
        .expanded(folder.expanded)
        .has_children(folder.has_children && !folder.is_source_root)
        .selected(folder.selected)
        .focused(folder.focused)
        .drag_drop_state(folder_tree_drag_drop_state(folder))
        .row_height(TREE_ROW_HEIGHT)
        .expander_width(FOLDER_EXPANDER_WIDTH)
        .style(SIDEBAR_ROW_STYLE)
        .guide_style(folder_tree_guide_style())
        .selected_hover_marker(folder_tree_selected_hover_marker())
        .highlighted_label_color(folder_tree_highlighted_label_color(folder));

    let row = if let Some(label_color) = folder_tree_label_color(folder) {
        row.label_color(label_color)
    } else {
        row
    };
    let row = if folder.locked {
        row.trailing_icon(folder_lock_icon(folder.lock_inherited))
    } else {
        row
    };

    row.row_key(format!("folder-row-{id}"))
        .hit_key(format!("folder-row-hit-{id}"))
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
    ui::row_actions()
        .primary_with_modifiers_key(id.clone(), |id, modifiers| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id, modifiers))
        })
        .double_key(id.clone(), |id| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id, Default::default()))
        })
        .secondary_key(id.clone(), |id, position| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::OpenFolderContextMenu(id, position))
        })
        .drag_key(id.clone(), |id, drag| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(id, drag))
        })
        .drop_target_key(
            id,
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

fn folder_tree_label_color(folder: &VisibleFolder) -> Option<ui::Rgba8> {
    folder.empty.then_some(FOLDER_TREE_EMPTY_LABEL)
}

fn folder_tree_highlighted_label_color(folder: &VisibleFolder) -> ui::Rgba8 {
    if folder.empty {
        FOLDER_TREE_EMPTY_LABEL
    } else {
        FOLDER_TREE_HIGHLIGHTED_LABEL
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

fn folder_tree_guide_style() -> ui::StyledTreeGuideStyle {
    ui::StyledTreeGuideStyle::new(TREE_DEPTH_INDENT, TREE_ROW_HEIGHT, SIDEBAR_ROW_STYLE)
}

#[cfg(test)]
fn folder_tree_palette_for_tests(theme: &ui::ThemeTokens) -> ui::DenseRowPalette {
    ui::dense_row_palette_from_style(theme, SIDEBAR_ROW_STYLE)
}

fn folder_tree_selected_hover_marker() -> ui::DenseRowMarkerStyle {
    ui::DenseRowMarkerStyle::new(
        ui::DenseRowMarkerParts::leading(FOLDER_TREE_SELECTED_HOVER_MARKER_WIDTH)
            .edge_inset(1.0)
            .vertical_inset(3.0),
        ui::ThemeTokens::default()
            .accent_mint
            .with_alpha(FOLDER_TREE_SELECTED_HOVER_MARKER_ALPHA),
    )
}

fn folder_lock_icon(inherited: bool) -> ui::SvgIcon {
    let color = if inherited {
        FOLDER_LOCK_INHERITED_ICON_COLOR
    } else {
        FOLDER_LOCK_ICON_COLOR
    };
    FOLDER_LOCK_ICON.icon(color)
}

static FOLDER_LOCK_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M4.1 7.1V5.6C4.1 3.45 5.65 2 8 2s3.9 1.45 3.9 3.6v1.5" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"/>
  <rect x="3" y="6.75" width="10" height="7" rx="1.2" fill="currentColor"/>
  <rect x="7.3" y="9" width="1.4" height="2.7" rx=".55" fill="rgb(24,24,24)"/>
</svg>"#,
);

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

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn folder_tree_uses_shared_grey_sidebar_hover_fill() {
        let theme = ui::ThemeTokens::default();
        let palette = folder_tree_palette_for_tests(&theme);
        let expected = sidebar_row_full_palette(&theme);

        assert_eq!(palette.hovered, expected.hovered);
        assert_eq!(palette.candidate_hovered, expected.candidate_hovered);
        assert_eq!(palette.selected, expected.selected);
        assert_eq!(palette.selected_hovered, expected.selected_hovered);
    }

    #[test]
    fn folder_tree_selected_hover_marker_uses_left_orange_rail() {
        let marker = folder_tree_selected_hover_marker();

        assert_eq!(marker.parts.width, FOLDER_TREE_SELECTED_HOVER_MARKER_WIDTH);
        assert_eq!(
            marker.color,
            ui::ThemeTokens::default()
                .accent_mint
                .with_alpha(FOLDER_TREE_SELECTED_HOVER_MARKER_ALPHA)
        );
    }

    #[test]
    fn empty_folder_rows_use_subdued_idle_label_color() {
        let folder = visible_folder_for_tests(true);

        assert_eq!(
            folder_tree_label_color(&folder),
            Some(FOLDER_TREE_EMPTY_LABEL)
        );
    }

    #[test]
    fn non_empty_folder_rows_use_default_idle_label_color() {
        let folder = visible_folder_for_tests(false);

        assert_eq!(folder_tree_label_color(&folder), None);
    }

    #[test]
    fn selected_empty_folder_rows_keep_subdued_label_color() {
        let mut folder = visible_folder_for_tests(true);
        folder.selected = true;

        let frame = folder_row(&folder)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));

        assert_eq!(
            frame.paint_plan.first_text_color("Folder"),
            Some(FOLDER_TREE_EMPTY_LABEL)
        );
    }

    #[test]
    fn selected_non_empty_folder_rows_use_default_label_color() {
        let mut folder = visible_folder_for_tests(false);
        folder.selected = true;

        let frame = folder_row(&folder)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));

        assert_eq!(
            frame.paint_plan.first_text_color("Folder"),
            Some(ui::ThemeTokens::default().text_primary)
        );
    }

    #[test]
    fn focused_unselected_folder_rows_use_default_label_color() {
        let mut folder = visible_folder_for_tests(false);
        folder.focused = true;

        let frame = folder_row(&folder)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));

        assert_eq!(
            frame.paint_plan.first_text_color("Folder"),
            Some(ui::ThemeTokens::default().text_primary)
        );
    }

    #[test]
    fn focused_unselected_folder_rows_paint_selected_fill() {
        let mut folder = visible_folder_for_tests(false);
        folder.focused = true;

        let frame = folder_row(&folder)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));
        let selected_fill = folder_tree_palette_for_tests(&ui::ThemeTokens::default())
            .selected
            .expect("folder tree selected fill");

        assert!(
            frame.paint_plan.fill_rects().any(|fill| {
                fill.color == selected_fill && (fill.rect.height() - TREE_ROW_HEIGHT).abs() < 0.5
            }),
            "focused folder rows should paint the same base fill as selected source rows"
        );
    }

    #[test]
    fn focused_selected_folder_rows_paint_selected_fill_without_hover_marker() {
        let mut folder = visible_folder_for_tests(false);
        folder.focused = true;
        folder.selected = true;

        let frame = folder_row(&folder)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(220.0, TREE_ROW_HEIGHT));
        let selected_fill = folder_tree_palette_for_tests(&ui::ThemeTokens::default())
            .selected
            .expect("folder tree selected fill");
        let selected_hover_fill = folder_tree_palette_for_tests(&ui::ThemeTokens::default())
            .selected_hovered
            .expect("folder tree selected-hover fill");
        let marker = folder_tree_selected_hover_marker();

        assert!(
            frame
                .paint_plan
                .fill_rects()
                .any(|fill| fill.color == selected_fill),
            "focused selected folder rows should paint the base selected fill"
        );
        assert!(
            !frame
                .paint_plan
                .fill_rects()
                .any(|fill| fill.color == selected_hover_fill),
            "focused selected folder rows should reserve selected-hover fill for pointer hover"
        );
        assert!(
            !frame
                .paint_plan
                .fill_rects()
                .any(|fill| fill.color == marker.color && fill.rect.width() == marker.parts.width),
            "focused selected folder rows should reserve the selected-hover marker for pointer hover"
        );
    }

    fn visible_folder_for_tests(empty: bool) -> VisibleFolder {
        VisibleFolder {
            id: String::from("folder"),
            name: String::from("Folder"),
            depth: 0,
            is_source_root: false,
            has_children: false,
            empty,
            locked: false,
            lock_inherited: false,
            expanded: false,
            selected: false,
            focused: false,
            drag_active: false,
            drag_source: false,
            drop_candidate: false,
            drop_target: false,
            drop_target_active: false,
            rename_draft: None,
            rename_input_id: None,
        }
    }
}
