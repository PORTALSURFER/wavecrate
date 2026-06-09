use crate::native_app::app::NativeAppState;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::sample_library::folder_browser::FolderBrowserState;

pub(in crate::native_app) struct LibrarySidebarViewModel<'a> {
    pub(in crate::native_app) folder_browser: &'a mut FolderBrowserState,
    pub(in crate::native_app) sidebar_width: f32,
    pub(in crate::native_app) tag_editor: TagEditorViewModel<'a>,
}

pub(in crate::native_app) struct TagEditorViewModel<'a> {
    pub(in crate::native_app) has_selected_file: bool,
    pub(in crate::native_app) draft: &'a str,
    pub(in crate::native_app) tokens: &'a [String],
    pub(in crate::native_app) pending_category_tag: Option<String>,
    pub(in crate::native_app) input_placeholder: &'a str,
    pub(in crate::native_app) completion_suffix: Option<String>,
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) display_categories: Vec<MetadataTagDisplayCategory>,
    pub(in crate::native_app) selected_tag: Option<String>,
}

impl<'a> LibrarySidebarViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a mut NativeAppState) -> Self {
        let sidebar_width = state.ui.chrome.folder_panel.size();
        let has_selected_file = state.library.folder_browser.selected_file_id().is_some();
        let tag_editor = TagEditorViewModel {
            has_selected_file,
            draft: state.metadata.tag_draft.as_str(),
            tokens: state.metadata.tag_tokens.as_slice(),
            pending_category_tag: state
                .pending_metadata_tag_category_tag()
                .map(str::to_string),
            input_placeholder: state.metadata_tag_input_placeholder(),
            completion_suffix: state.metadata_tag_completion_suffix(),
            tags: state.selected_metadata_tags().to_vec(),
            display_categories: state.selected_metadata_tag_display_categories(),
            selected_tag: state.metadata.selected_tag.clone(),
        };

        Self {
            folder_browser: &mut state.library.folder_browser,
            sidebar_width,
            tag_editor,
        }
    }
}
