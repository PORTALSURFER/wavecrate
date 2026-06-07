use radiant::prelude as ui;

use crate::native_app::browser::folder_browser::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
};
use crate::native_app::metadata::{MetadataTagCompletionOption, MetadataTagDisplayCategory};

use super::tag_editor::{metadata_section, tag_field_height};
use super::tag_entry_layout::tag_field_content_width;
use super::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, TREE_ROW_HEIGHT, VisibleFolder, plural,
    tree_hit_target::FolderTreeHitTarget,
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
const FOLDER_TREE_GUIDE_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 126,
    b: 64,
    a: 152,
};

pub(in crate::native_app) fn folder_browser_view_mut(
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
        metadata_tag_input_placeholder,
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
pub(in crate::native_app) fn folder_browser_view(
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
    let drag_revision = state.drag_revision();
    let window = state.follow_selected_tree_view(
        &visible_folders,
        FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
        FOLDER_TREE_OVERSCAN_ROWS,
        FOLDER_TREE_EDGE_CONTEXT_ROWS,
    );
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
    ui::TreeGuideStyle::new(
        super::TREE_DEPTH_INDENT,
        TREE_ROW_HEIGHT,
        FOLDER_TREE_GUIDE_COLOR,
    )
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

fn selected_folder_status(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let Some(folder) = state.selected_folder() else {
        return ui::text_line("No folder selected", 20.0);
    };
    let file_count = state.selected_files().len();
    let audio_count = state.selected_folder_audio_file_count();
    let folder_name = if state.selected_folder_is_source_root() {
        "."
    } else {
        folder.name.as_str()
    };
    let label = format!(
        "{} | {audio_count} audio | {file_count} item{}",
        folder_name,
        plural(file_count)
    );
    ui::text_line(label, 20.0)
}
