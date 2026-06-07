use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::library_browser::folder_browser::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_LIST_ID, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS, FolderBrowserMessage, FolderBrowserState,
    TREE_DEPTH_INDENT, TREE_ROW_HEIGHT, VisibleFolder,
};
#[cfg(test)]
use crate::native_app::metadata::MetadataTagCompletionOption;
use crate::native_app::metadata::MetadataTagDisplayCategory;

use tag_editor::{metadata_section, tag_field_height};
use tree_hit_target::FolderTreeHitTarget;

mod collections_section;
mod filter_section;
mod source_section;
mod tag_completion;
mod tag_editor;
mod tag_entry_layout;
mod tree_hit_target;

use collections_section::collections_section;
use filter_section::filter_section;
use source_section::source_selector;

pub(in crate::native_app) use tag_completion::{TAG_COMPLETION_POPUP_GAP, tag_completion_overlay};
pub(in crate::native_app) use tag_editor::metadata_tag_completion_bottom_inset;
#[cfg(test)]
pub(in crate::native_app) use tag_editor::{
    METADATA_SIDEBAR_PANEL_ID, METADATA_TAG_INPUT_ID, METADATA_TAG_LIBRARY_TOGGLE_ID,
};
pub(in crate::native_app) use tag_entry_layout::tag_field_content_width;

const FOLDER_EXPANDER_WIDTH: f32 = 28.0;
const FOLDER_TREE_GUIDE_COLOR: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 126,
    b: 64,
    a: 152,
};

pub(in crate::native_app) struct FolderSidebarViewModel<'a> {
    folder_browser: &'a mut FolderBrowserState,
    sidebar_width: f32,
    has_selected_file: bool,
    metadata_tag_draft: &'a str,
    metadata_tag_tokens: &'a [String],
    metadata_tag_pending_category_tag: Option<String>,
    metadata_tag_input_placeholder: &'static str,
    metadata_tag_completion_suffix: Option<String>,
    metadata_tags: Vec<String>,
    metadata_tag_display_categories: Vec<MetadataTagDisplayCategory>,
    selected_metadata_tag: Option<String>,
}

impl<'a> FolderSidebarViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a mut NativeAppState) -> Self {
        let sidebar_width = state.folder_panel.size();
        let has_selected_file = state.folder_browser.selected_file_id().is_some();
        let metadata_tag_pending_category_tag = state
            .pending_metadata_tag_category_tag()
            .map(str::to_string);
        let metadata_tag_completion_suffix = state.metadata_tag_completion_suffix();
        let metadata_tags = state.selected_metadata_tags().to_vec();
        let metadata_tag_display_categories = state.selected_metadata_tag_display_categories();
        let selected_metadata_tag = state.selected_metadata_tag.clone();
        let metadata_tag_input_placeholder = state.metadata_tag_input_placeholder();

        Self {
            folder_browser: &mut state.folder_browser,
            sidebar_width,
            has_selected_file,
            metadata_tag_draft: state.metadata_tag_draft.as_str(),
            metadata_tag_tokens: state.metadata_tag_tokens.as_slice(),
            metadata_tag_pending_category_tag,
            metadata_tag_input_placeholder,
            metadata_tag_completion_suffix,
            metadata_tags,
            metadata_tag_display_categories,
            selected_metadata_tag,
        }
    }
}

pub(in crate::native_app) fn folder_sidebar(
    model: FolderSidebarViewModel<'_>,
) -> ui::View<GuiMessage> {
    let sidebar_width = model.sidebar_width;
    folder_browser_view_mut(
        model.folder_browser,
        sidebar_width,
        model.has_selected_file,
        model.metadata_tag_draft,
        model.metadata_tag_tokens,
        model.metadata_tag_pending_category_tag.as_deref(),
        model.metadata_tag_input_placeholder,
        model.metadata_tag_completion_suffix.as_deref(),
        model.metadata_tags.as_slice(),
        model.metadata_tag_display_categories.as_slice(),
        model.selected_metadata_tag.as_deref(),
    )
    .width(sidebar_width)
    .fill_height()
}

pub(in crate::native_app) fn folder_browser_view_mut(
    state: &mut FolderBrowserState,
    sidebar_width: f32,
    has_selected_file: bool,
    metadata_tag_draft: &str,
    metadata_tag_tokens: &[String],
    metadata_tag_pending_category_tag: Option<&str>,
    metadata_tag_input_placeholder: &str,
    metadata_tag_completion_suffix: Option<&str>,
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
    _metadata_tag_completion_options: &[MetadataTagCompletionOption],
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

fn selected_folder_status(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    ui::text_line(state.selected_folder_status_label(), 20.0)
}
