use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::{
    LibrarySidebarViewModel, TagEditorViewModel,
};
#[cfg(test)]
use crate::native_app::metadata::MetadataTagCompletionOption;
#[cfg(test)]
use crate::native_app::metadata::MetadataTagDisplayCategory;
#[cfg(test)]
use crate::native_app::sample_library::folder_browser::FolderBrowserState;

use tag_editor::{metadata_section, tag_field_height};

mod collections_section;
mod filter_section;
mod folder_tree;
mod source_section;
mod tag_completion;
mod tag_editor;
mod tag_entry_layout;
mod tree_hit_target;

use collections_section::collections_section;
use filter_section::filter_section;
use folder_tree::folder_tree_section;
use source_section::source_selector;

pub(in crate::native_app) use tag_completion::{TAG_COMPLETION_POPUP_GAP, tag_completion_overlay};
pub(in crate::native_app) use tag_editor::metadata_tag_completion_bottom_inset;
#[cfg(test)]
pub(in crate::native_app) use tag_editor::{
    METADATA_SIDEBAR_PANEL_ID, METADATA_TAG_INPUT_ID, METADATA_TAG_LIBRARY_TOGGLE_ID,
};
pub(in crate::native_app) use tag_entry_layout::tag_field_content_width;

pub(in crate::native_app) fn library_sidebar(
    model: LibrarySidebarViewModel<'_>,
) -> ui::View<GuiMessage> {
    let sidebar_width = model.sidebar_width;
    library_sidebar_content(model)
        .width(sidebar_width)
        .fill_height()
}

fn library_sidebar_content(model: LibrarySidebarViewModel<'_>) -> ui::View<GuiMessage> {
    ui::column([
        source_selector(&*model.folder_browser),
        folder_tree_section(&mut *model.folder_browser),
        collections_section(&*model.folder_browser),
        filter_section(&*model.folder_browser),
        tag_editor_section(
            &model.tag_editor,
            model.sidebar_width,
            model.folder_browser.metadata_panel_height(),
        ),
    ])
    .spacing(3.0)
    .fill_width()
    .padding_x(4.0)
    .style(ui::WidgetStyle::default())
    .fill_height()
}

fn tag_editor_section(
    model: &TagEditorViewModel<'_>,
    sidebar_width: f32,
    panel_height: f32,
) -> ui::View<GuiMessage> {
    let content_width = tag_field_content_width(sidebar_width);
    let field_height = tag_field_height(
        model.draft,
        model.tokens,
        model.pending_category_tag.as_deref(),
        model.input_placeholder,
        model.completion_suffix.as_deref(),
        model.tags.as_slice(),
        model.display_categories.as_slice(),
        content_width,
    );
    metadata_section(
        model.draft,
        model.tokens,
        model.pending_category_tag.as_deref(),
        model.input_placeholder,
        model.completion_suffix.as_deref(),
        model.tags.as_slice(),
        model.display_categories.as_slice(),
        model.selected_tag.as_deref(),
        content_width,
        field_height,
        panel_height,
        model.has_selected_file,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(in crate::native_app) fn library_sidebar_view(
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
    library_sidebar_content(LibrarySidebarViewModel {
        folder_browser: &mut state,
        sidebar_width,
        tag_editor: TagEditorViewModel {
            has_selected_file,
            draft: metadata_tag_draft,
            tokens: metadata_tag_tokens,
            pending_category_tag: metadata_tag_pending_category_tag.map(str::to_string),
            input_placeholder: metadata_tag_input_placeholder,
            completion_suffix: metadata_tag_completion_suffix.map(str::to_string),
            tags: metadata_tags.to_vec(),
            display_categories: metadata_tag_display_categories.to_vec(),
            selected_tag: selected_metadata_tag.map(str::to_string),
        },
    })
}
