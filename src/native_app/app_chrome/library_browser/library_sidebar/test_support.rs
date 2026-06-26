use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::{
    CollectionsSectionViewModel, FilterSectionViewModel, FolderTreeViewModel,
    LibrarySidebarViewModel, SourceSelectorViewModel, TagEditorViewModel,
};
use crate::native_app::metadata::{MetadataTagCompletionOption, MetadataTagDisplayCategory};
use crate::native_app::sample_library::folder_browser::FolderBrowserState;
use crate::native_app::sample_library::folder_browser::view_contract::{
    FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_OVERSCAN_ROWS, FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
};

use super::library_sidebar_content;

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
    let visible_folders = state.visible_folders();
    let tree_window = state.tree_view_window(
        &visible_folders,
        FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
        FOLDER_TREE_OVERSCAN_ROWS,
        FOLDER_TREE_EDGE_CONTEXT_ROWS,
    );
    library_sidebar_content(LibrarySidebarViewModel {
        sidebar_width,
        metadata_panel_height: state.metadata_panel_height(),
        source_selector: SourceSelectorViewModel::from_folder_browser(state),
        folder_tree: FolderTreeViewModel {
            visible_folders,
            window: tree_window,
            selected_folder_status_label: state.selected_folder_status_label(),
            selected_source_missing: state.source_is_missing(state.selected_source_id()),
            include_subfolders_available: state.folder_subtree_listing_available(),
            include_subfolders: state.folder_subtree_listing_enabled(),
            show_empty_folders: state.empty_folder_visibility_enabled(),
            help_tooltips_enabled: false,
        },
        collections: CollectionsSectionViewModel::from_folder_browser(state),
        filter: FilterSectionViewModel::from_folder_browser(state, false),
        harvest_family: None,
        tag_editor: TagEditorViewModel {
            has_selected_file,
            draft: metadata_tag_draft.to_string(),
            tokens: metadata_tag_tokens.to_vec(),
            pending_category_tag: metadata_tag_pending_category_tag.map(str::to_string),
            input_placeholder: metadata_tag_input_placeholder.to_string(),
            completion_suffix: metadata_tag_completion_suffix.map(str::to_string),
            tags: metadata_tags.to_vec(),
            mixed_tags: Vec::new(),
            display_categories: metadata_tag_display_categories.to_vec(),
            selected_tag: selected_metadata_tag.map(str::to_string),
        },
    })
}
