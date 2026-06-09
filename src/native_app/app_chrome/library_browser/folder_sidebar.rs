use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::folder_sidebar::FolderSidebarViewModel;
#[cfg(test)]
use crate::native_app::metadata::MetadataTagCompletionOption;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::sample_library::folder_browser::FolderBrowserState;

use tag_editor::{metadata_section, tag_field_height};

mod collections_section;
mod filter_section;
mod folder_tree_section;
mod source_section;
mod tag_completion;
mod tag_editor;
mod tag_entry_layout;
mod tree_hit_target;

use collections_section::collections_section;
use filter_section::filter_section;
use folder_tree_section::folders_section;
use source_section::source_selector;

pub(in crate::native_app) use tag_completion::{TAG_COMPLETION_POPUP_GAP, tag_completion_overlay};
pub(in crate::native_app) use tag_editor::metadata_tag_completion_bottom_inset;
#[cfg(test)]
pub(in crate::native_app) use tag_editor::{
    METADATA_SIDEBAR_PANEL_ID, METADATA_TAG_INPUT_ID, METADATA_TAG_LIBRARY_TOGGLE_ID,
};
pub(in crate::native_app) use tag_entry_layout::tag_field_content_width;

pub(in crate::native_app) fn folder_sidebar(
    model: FolderSidebarViewModel<'_>,
) -> ui::View<GuiMessage> {
    let sidebar_width = model.sidebar_width;
    folder_sidebar_content(FolderSidebarContent::from_view_model(model))
        .width(sidebar_width)
        .fill_height()
}

struct FolderSidebarContent<'a> {
    folder_browser: &'a mut FolderBrowserState,
    sidebar_width: f32,
    has_selected_file: bool,
    metadata_tag_draft: &'a str,
    metadata_tag_tokens: &'a [String],
    metadata_tag_pending_category_tag: Option<String>,
    metadata_tag_input_placeholder: &'a str,
    metadata_tag_completion_suffix: Option<String>,
    metadata_tags: Vec<String>,
    metadata_tag_display_categories: Vec<MetadataTagDisplayCategory>,
    selected_metadata_tag: Option<String>,
}

impl<'a> FolderSidebarContent<'a> {
    fn from_view_model(model: FolderSidebarViewModel<'a>) -> Self {
        Self {
            folder_browser: model.folder_browser,
            sidebar_width: model.sidebar_width,
            has_selected_file: model.has_selected_file,
            metadata_tag_draft: model.metadata_tag_draft,
            metadata_tag_tokens: model.metadata_tag_tokens,
            metadata_tag_pending_category_tag: model.metadata_tag_pending_category_tag,
            metadata_tag_input_placeholder: model.metadata_tag_input_placeholder,
            metadata_tag_completion_suffix: model.metadata_tag_completion_suffix,
            metadata_tags: model.metadata_tags,
            metadata_tag_display_categories: model.metadata_tag_display_categories,
            selected_metadata_tag: model.selected_metadata_tag,
        }
    }

    fn tag_field_content_width(&self) -> f32 {
        tag_field_content_width(self.sidebar_width)
    }

    fn tag_field_height(&self) -> f32 {
        tag_field_height(
            self.metadata_tag_draft,
            self.metadata_tag_tokens,
            self.metadata_tag_pending_category_tag.as_deref(),
            self.metadata_tag_input_placeholder,
            self.metadata_tag_completion_suffix.as_deref(),
            self.metadata_tags.as_slice(),
            self.metadata_tag_display_categories.as_slice(),
            self.tag_field_content_width(),
        )
    }
}

fn folder_sidebar_content(content: FolderSidebarContent<'_>) -> ui::View<GuiMessage> {
    ui::column([
        source_selector(&*content.folder_browser),
        folders_section(&mut *content.folder_browser),
        collections_section(&*content.folder_browser),
        filter_section(&*content.folder_browser),
        metadata_section_for_sidebar(&content),
    ])
    .spacing(3.0)
    .fill_width()
    .padding_x(4.0)
    .style(ui::WidgetStyle::default())
    .fill_height()
}

fn metadata_section_for_sidebar(content: &FolderSidebarContent<'_>) -> ui::View<GuiMessage> {
    metadata_section(
        content.metadata_tag_draft,
        content.metadata_tag_tokens,
        content.metadata_tag_pending_category_tag.as_deref(),
        content.metadata_tag_input_placeholder,
        content.metadata_tag_completion_suffix.as_deref(),
        content.metadata_tags.as_slice(),
        content.metadata_tag_display_categories.as_slice(),
        content.selected_metadata_tag.as_deref(),
        content.tag_field_content_width(),
        content.tag_field_height(),
        content.folder_browser.metadata_panel_height(),
        content.has_selected_file,
    )
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
    folder_sidebar_content(FolderSidebarContent {
        folder_browser: &mut state,
        sidebar_width,
        has_selected_file,
        metadata_tag_draft,
        metadata_tag_tokens,
        metadata_tag_pending_category_tag: metadata_tag_pending_category_tag.map(str::to_string),
        metadata_tag_input_placeholder,
        metadata_tag_completion_suffix: metadata_tag_completion_suffix.map(str::to_string),
        metadata_tags: metadata_tags.to_vec(),
        metadata_tag_display_categories: metadata_tag_display_categories.to_vec(),
        selected_metadata_tag: selected_metadata_tag.map(str::to_string),
    })
}
