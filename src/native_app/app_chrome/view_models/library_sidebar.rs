use radiant::prelude as ui;

use crate::native_app::app::NativeAppState;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_OVERSCAN_ROWS, FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
};
use crate::native_app::sample_library::folder_browser::{FolderBrowserState, model::VisibleFolder};

pub(in crate::native_app) struct LibrarySidebarViewModel {
    pub(in crate::native_app) sidebar_width: f32,
    pub(in crate::native_app) metadata_panel_height: f32,
    pub(in crate::native_app) folder_tree: FolderTreeViewModel,
    pub(in crate::native_app) tag_editor: TagEditorViewModel,
}

pub(in crate::native_app) struct FolderTreeViewModel {
    pub(in crate::native_app) visible_folders: Vec<VisibleFolder>,
    pub(in crate::native_app) window: ui::VirtualListWindow,
    pub(in crate::native_app) drag_revision: u64,
    pub(in crate::native_app) selected_folder_status_label: String,
}

pub(in crate::native_app) struct TagEditorViewModel {
    pub(in crate::native_app) has_selected_file: bool,
    pub(in crate::native_app) draft: String,
    pub(in crate::native_app) tokens: Vec<String>,
    pub(in crate::native_app) pending_category_tag: Option<String>,
    pub(in crate::native_app) input_placeholder: String,
    pub(in crate::native_app) completion_suffix: Option<String>,
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) display_categories: Vec<MetadataTagDisplayCategory>,
    pub(in crate::native_app) selected_tag: Option<String>,
}

impl LibrarySidebarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        let folder_browser = &state.library.folder_browser;
        Self {
            sidebar_width: state.ui.chrome.folder_panel.size(),
            metadata_panel_height: folder_browser.metadata_panel_height(),
            folder_tree: FolderTreeViewModel::from_folder_browser(folder_browser),
            tag_editor: TagEditorViewModel::from_app_state(state),
        }
    }
}

impl FolderTreeViewModel {
    fn from_folder_browser(folder_browser: &FolderBrowserState) -> Self {
        let visible_folders = folder_browser.visible_folders();
        let window = folder_browser.tree_view_window(
            &visible_folders,
            FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
            FOLDER_TREE_OVERSCAN_ROWS,
            FOLDER_TREE_EDGE_CONTEXT_ROWS,
        );

        Self {
            visible_folders,
            window,
            drag_revision: folder_browser.drag_revision(),
            selected_folder_status_label: folder_browser.selected_folder_status_label(),
        }
    }
}

impl TagEditorViewModel {
    fn from_app_state(state: &NativeAppState) -> Self {
        Self {
            has_selected_file: state.library.folder_browser.selected_file_id().is_some(),
            draft: state.metadata.tag_draft.clone(),
            tokens: state.metadata.tag_tokens.clone(),
            pending_category_tag: state
                .pending_metadata_tag_category_tag()
                .map(str::to_string),
            input_placeholder: state.metadata_tag_input_placeholder().to_string(),
            completion_suffix: state.metadata_tag_completion_suffix(),
            tags: state.selected_metadata_tags().to_vec(),
            display_categories: state.selected_metadata_tag_display_categories(),
            selected_tag: state.metadata.selected_tag.clone(),
        }
    }
}
