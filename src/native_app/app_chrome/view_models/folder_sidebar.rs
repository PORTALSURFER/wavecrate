use crate::native_app::app::NativeAppState;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::sample_library::folder_browser::FolderBrowserState;

pub(in crate::native_app) struct FolderSidebarViewModel<'a> {
    pub(in crate::native_app) folder_browser: &'a mut FolderBrowserState,
    pub(in crate::native_app) sidebar_width: f32,
    pub(in crate::native_app) has_selected_file: bool,
    pub(in crate::native_app) metadata_tag_draft: &'a str,
    pub(in crate::native_app) metadata_tag_tokens: &'a [String],
    pub(in crate::native_app) metadata_tag_pending_category_tag: Option<String>,
    pub(in crate::native_app) metadata_tag_input_placeholder: &'static str,
    pub(in crate::native_app) metadata_tag_completion_suffix: Option<String>,
    pub(in crate::native_app) metadata_tags: Vec<String>,
    pub(in crate::native_app) metadata_tag_display_categories: Vec<MetadataTagDisplayCategory>,
    pub(in crate::native_app) selected_metadata_tag: Option<String>,
}

impl<'a> FolderSidebarViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a mut NativeAppState) -> Self {
        let sidebar_width = state.chrome.folder_panel.size();
        let has_selected_file = state.folder_browser.selected_file_id().is_some();
        let metadata_tag_pending_category_tag = state
            .pending_metadata_tag_category_tag()
            .map(str::to_string);
        let metadata_tag_completion_suffix = state.metadata_tag_completion_suffix();
        let metadata_tags = state.selected_metadata_tags().to_vec();
        let metadata_tag_display_categories = state.selected_metadata_tag_display_categories();
        let selected_metadata_tag = state.metadata.selected_tag.clone();
        let metadata_tag_input_placeholder = state.metadata_tag_input_placeholder();

        Self {
            folder_browser: &mut state.folder_browser,
            sidebar_width,
            has_selected_file,
            metadata_tag_draft: state.metadata.tag_draft.as_str(),
            metadata_tag_tokens: state.metadata.tag_tokens.as_slice(),
            metadata_tag_pending_category_tag,
            metadata_tag_input_placeholder,
            metadata_tag_completion_suffix,
            metadata_tags,
            metadata_tag_display_categories,
            selected_metadata_tag,
        }
    }
}
