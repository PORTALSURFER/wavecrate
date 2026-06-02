use radiant::prelude as ui;

use crate::gui_app::metadata_tags::{MetadataTagCompletionOption, MetadataTagDisplayCategory};
use crate::gui_app::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
};

use super::tag_editor::{metadata_section, tag_field_content_width, tag_field_height};
use super::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, TREE_DEPTH_INDENT, TREE_ROW_HEIGHT,
    VisibleFolder, plural,
    tree_hit_target::{FolderTreeHitMessage, FolderTreeHitTarget},
};

mod collections_section;
mod source_section;

use collections_section::collections_section;
use source_section::source_selector;

pub(in crate::gui_app) fn folder_browser_view_mut(
    state: &mut FolderBrowserState,
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
    ui::column([
        source_selector(state),
        ui::text("Folders").height(22.0).fill_width(),
        folder_tree_view(state),
        selected_folder_status(state),
        collections_section(state),
        filter_section(),
        metadata_section(
            metadata_tag_draft,
            metadata_tag_tokens,
            metadata_tag_pending_category_tag,
            metadata_tag_input_placeholder,
            metadata_tag_completion_suffix,
            metadata_tag_completion_options,
            metadata_tags,
            metadata_tag_display_categories,
            selected_metadata_tag,
            tag_field_content_width,
            tag_field_height,
            has_selected_file,
        ),
    ])
    .spacing(3.0)
    .padding(4.0)
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
    let window = state.follow_selected_tree_view(
        visible_folders.len(),
        selected_index,
        FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
        FOLDER_TREE_OVERSCAN_ROWS,
        FOLDER_TREE_EDGE_CONTEXT_ROWS,
    );
    ui::stack([
        ui::pointer_move_shield(state.drop_target_folder.is_some())
            .on_pointer_move(|position| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ClearDropTarget(position))
            })
            .key("folder-drop-clear-target")
            .input_only()
            .fill(),
        ui::virtual_list_window(
            window,
            TREE_ROW_HEIGHT,
            move |index| folder_row(visible_folders[index].clone()),
            TREE_ROW_HEIGHT * FOLDER_TREE_OVERSCAN_ROWS as f32,
        )
        .id(FOLDER_TREE_LIST_ID)
        .fill_width()
        .fill_height(),
    ])
    .fill()
}

fn folder_row(folder: VisibleFolder) -> ui::View<GuiMessage> {
    let id = folder.id.clone();
    if let (Some(draft), Some(input_id)) = (folder.rename_draft.clone(), folder.rename_input_id) {
        let caret = draft.chars().count();
        let indent = (folder.depth as f32) * TREE_DEPTH_INDENT;
        return ui::row([
            ui::spacer().width(indent).height(22.0),
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
        .style(ui::WidgetStyle::new(
            ui::WidgetTone::Accent,
            ui::WidgetProminence::Subtle,
        ))
        .fill_width()
        .height(TREE_ROW_HEIGHT)
        .spacing(1.0)
        .hoverable();
    }

    let expander = if folder.expanded { "[-]" } else { "[+]" };
    let indent = (folder.depth as f32) * TREE_DEPTH_INDENT;
    let label_text = if folder.has_children {
        format!("{expander} {}", folder.name)
    } else {
        format!("    {}", folder.name)
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
    .key(format!("folder-row-hit-{id}"))
    .fill_width()
    .height(22.0);

    ui::row([
        ui::spacer().width(indent).height(22.0),
        hit_target.fill_width().height(22.0),
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.selected || folder.drop_target {
        ui::WidgetStyle::new(ui::WidgetTone::Accent, ui::WidgetProminence::Subtle)
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
            format!(
                "{} | {audio_count} audio | {file_count} item{}",
                folder.name,
                plural(file_count)
            )
        })
        .unwrap_or_else(|| String::from("No folder selected"));
    ui::text(label).height(20.0).fill_width().truncate()
}

fn filter_section() -> ui::View<GuiMessage> {
    ui::property_panel(
        "Filter",
        [
            ui::PropertyRow::new("name", "Name", "Any"),
            ui::PropertyRow::new("type", "Type", "Audio"),
        ],
    )
    .height(76.0)
}
