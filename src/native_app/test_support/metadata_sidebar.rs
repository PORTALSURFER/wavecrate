use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar;
use crate::native_app::metadata::{MetadataTagCompletionOption, MetadataTagDisplayCategory};
use crate::native_app::sample_library::folder_browser::FolderBrowserState;
use crate::native_app::ui::ids;
use radiant::prelude as ui;

pub(in crate::native_app) const METADATA_SIDEBAR_PANEL_ID: u64 = ids::METADATA_SIDEBAR_PANEL_ID;
pub(in crate::native_app) const METADATA_TAG_INPUT_ID: u64 = ids::METADATA_TAG_INPUT_ID;
pub(in crate::native_app) const METADATA_TAG_LIBRARY_TOGGLE_ID: u64 =
    ids::METADATA_TAG_LIBRARY_TOGGLE_ID;
pub(in crate::native_app) const METADATA_RESIZE_HEADER_ID: u64 = ids::METADATA_RESIZE_HEADER_ID;

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
    metadata_tag_completion_options: &[MetadataTagCompletionOption],
    metadata_tags: &[String],
    metadata_tag_display_categories: &[MetadataTagDisplayCategory],
    selected_metadata_tag: Option<&str>,
) -> ui::View<GuiMessage> {
    library_sidebar::library_sidebar_view(
        state,
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
