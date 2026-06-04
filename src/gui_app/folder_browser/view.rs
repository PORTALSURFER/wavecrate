use radiant::prelude as ui;

use crate::gui_app::metadata_tags::{MetadataTagCompletionOption, MetadataTagDisplayCategory};
use crate::gui_app::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
};

use super::tag_editor::{metadata_section, tag_field_height};
use super::tag_entry_layout::tag_field_content_width;
use super::{
    FolderBrowserDropTarget, FolderBrowserMessage, FolderBrowserState, GuiMessage, TREE_ROW_HEIGHT,
    VisibleFolder, plural, tree_guides,
    tree_hit_target::{FolderTreeHitMessage, FolderTreeHitTarget},
};

mod collections_section;
mod filter_section;
mod source_section;

use collections_section::collections_section;
#[cfg(test)]
pub(super) use filter_section::COLLAPSED_FILTER_PANEL_HEIGHT;
pub(super) use filter_section::DEFAULT_FILTER_PANEL_HEIGHT;
use filter_section::filter_section;
use source_section::source_selector;

const FOLDER_EXPANDER_WIDTH: f32 = 28.0;

pub(in crate::gui_app) fn folder_browser_view_mut(
    state: &mut FolderBrowserState,
    sidebar_width: f32,
    has_selected_file: bool,
    metadata_tag_draft: &str,
    metadata_tag_tokens: &[String],
    metadata_tag_pending_category_tag: Option<&str>,
    metadata_tag_input_placeholder: &str,
    metadata_tag_completion_suffix: Option<&str>,
    _metadata_tag_completion_options: &[MetadataTagCompletionOption],
    metadata_tags: &[String],
    metadata_tag_display_categories: &[MetadataTagDisplayCategory],
    selected_metadata_tag: Option<&str>,
) -> ui::View<GuiMessage> {
    let tag_field_content_width = tag_field_content_width(sidebar_width);
    let tag_field_height = tag_field_height(
        metadata_tag_draft,
        metadata_tag_tokens,
        metadata_tag_pending_category_tag,
        metadata_tag_completion_suffix,
        metadata_tags,
        metadata_tag_display_categories,
        tag_field_content_width,
    );
    let content = ui::column([
        source_selector(state),
        ui::text_line("Folders", 22.0),
        folder_tree_view(state),
        selected_folder_status(state),
        collections_section(state),
        filter_section(state),
        metadata_section(
            metadata_tag_draft,
            metadata_tag_tokens,
            metadata_tag_pending_category_tag,
            metadata_tag_input_placeholder,
            metadata_tag_completion_suffix,
            metadata_tags,
            metadata_tag_display_categories,
            selected_metadata_tag,
            tag_field_content_width,
            tag_field_height,
            state.metadata_panel_height(),
            has_selected_file,
        ),
    ])
    .spacing(3.0)
    .fill_width()
    .fill_height();
    ui::column([ui::spacer().height(4.0).fill_width(), content])
        .spacing(0.0)
        .padding_x(4.0)
        .style(ui::WidgetStyle::default())
        .fill_height()
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(in crate::gui_app) fn folder_browser_view(
    state: &FolderBrowserState,
    sidebar_width: f32,
    has_selected_file: bool,
    metadata_tag_draft: &str,
    metadata_tag_tokens: &[String],
    metadata_tag_pending_category_tag: Option<&str>,
    metadata_tag_input_placeholder: &str,
    metadata_tag_completion_suffix: Option<&str>,
    metadata_tag_completion_options: &[MetadataTagCompletionOption],
    metadata_tags: &[String],
    metadata_tag_display_categories: &[MetadataTagDisplayCategory],
    selected_metadata_tag: Option<&str>,
) -> ui::View<GuiMessage> {
    let mut state = state.clone();
    folder_browser_view_mut(
        &mut state,
        sidebar_width,
        has_selected_file,
        metadata_tag_draft,
        metadata_tag_tokens,
        metadata_tag_pending_category_tag,
        metadata_tag_input_placeholder,
        metadata_tag_completion_suffix,
        metadata_tag_completion_options,
        metadata_tags,
        metadata_tag_display_categories,
        selected_metadata_tag,
    )
}

fn folder_tree_view(state: &mut FolderBrowserState) -> ui::View<GuiMessage> {
    let visible_folders = state.visible_folders();
    let selected_index = visible_folders.iter().position(|folder| folder.selected);
    let drag_revision = state.drag_revision();
    let window = state.follow_selected_tree_view(
        visible_folders.len(),
        selected_index,
        FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
        FOLDER_TREE_OVERSCAN_ROWS,
        FOLDER_TREE_EDGE_CONTEXT_ROWS,
    );
    ui::stack([
        ui::pointer_move_shield(matches!(
            state.drop_target.current(),
            Some(FolderBrowserDropTarget::Folder(_))
        ))
        .on_pointer_move(|position| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ClearDropTarget(position))
        })
        .key("folder-drop-clear-target")
        .input_only()
        .fill(),
        folder_tree_window(visible_folders, window, drag_revision)
            .id(FOLDER_TREE_LIST_ID)
            .fill_width()
            .fill_height(),
    ])
    .fill()
}

fn folder_tree_window(
    visible_folders: Vec<VisibleFolder>,
    window: ui::VirtualListWindow,
    drag_revision: u64,
) -> ui::View<GuiMessage> {
    let row_height = TREE_ROW_HEIGHT;
    let projected_len = window.window_len();
    let mut children = Vec::with_capacity(projected_len + 2);

    let top_spacer_height = row_height * window.window_start as f32;
    if top_spacer_height > 0.0 {
        children.push(ui::spacer().height(top_spacer_height).fill_width());
    }

    if projected_len > 0 {
        let rows = ui::column((window.window_start..window.window_end).map(|index| {
            folder_row(visible_folders[index].clone(), drag_revision).height(row_height)
        }))
        .spacing(0.0)
        .fill_width()
        .height(row_height * projected_len as f32);
        children.push(
            ui::stack([
                rows,
                tree_guides::folder_tree_guides_overlay(
                    &visible_folders,
                    window.window_start,
                    window.window_end,
                ),
            ])
            .fill_width()
            .height(row_height * projected_len as f32),
        );
    }

    let bottom_items = window.total_items.saturating_sub(window.window_end);
    let bottom_spacer_height = row_height * bottom_items as f32;
    if bottom_spacer_height > 0.0 {
        children.push(ui::spacer().height(bottom_spacer_height).fill_width());
    }

    ui::virtual_scroll(
        ui::column(children).spacing(0.0).fill_width(),
        TREE_ROW_HEIGHT * FOLDER_TREE_OVERSCAN_ROWS as f32,
    )
    .style(ui::WidgetStyle::default())
    .fill_height()
}

fn folder_row(folder: VisibleFolder, drag_revision: u64) -> ui::View<GuiMessage> {
    let id = folder.id.clone();
    if let (Some(draft), Some(input_id)) = (folder.rename_draft.clone(), folder.rename_input_id) {
        let caret = draft.chars().count();
        return ui::row([
            tree_guides::folder_tree_indent(folder.depth),
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
    let hit_id = id.clone();
    let hit_target = ui::custom_widget_mapped(
        FolderTreeHitTarget::new(
            label_text,
            folder.selected,
            folder.drop_target,
            folder.drag_active,
            folder.drag_source,
            folder.drop_candidate,
            folder.drop_target_active,
        ),
        move |message| match message {
            FolderTreeHitMessage::Activate => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(hit_id.clone()))
            }
            FolderTreeHitMessage::ContextMenu(position) => GuiMessage::FolderBrowser(
                FolderBrowserMessage::OpenFolderContextMenu(hit_id.clone(), position),
            ),
            FolderTreeHitMessage::Drag(drag) => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(hit_id.clone(), drag))
            }
            FolderTreeHitMessage::Drop => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnFolder(hit_id.clone()))
            }
            FolderTreeHitMessage::HoverDropTarget(position) => GuiMessage::FolderBrowser(
                FolderBrowserMessage::HoverDropTarget(hit_id.clone(), position),
            ),
        },
    )
    .key(format!("folder-row-hit-{id}-{drag_revision}"))
    .fill_width()
    .height(22.0);

    ui::row([
        tree_guides::folder_tree_indent(folder.depth),
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

fn selected_folder_status(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let file_count = state.selected_files().len();
    let audio_count = state.selected_audio_files().len();
    let label = state
        .selected_folder()
        .map(|folder| {
            let folder_name = if state.selected_folder_is_source_root() {
                "."
            } else {
                folder.name.as_str()
            };
            format!(
                "{} | {audio_count} audio | {file_count} item{}",
                folder_name,
                plural(file_count)
            )
        })
        .unwrap_or_else(|| String::from("No folder selected"));
    ui::text_line(label, 20.0)
}
