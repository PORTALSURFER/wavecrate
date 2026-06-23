use radiant::prelude as ui;

use crate::native_app::app::NativeAppState;
use crate::native_app::metadata::MetadataTagDisplayCategory;
use crate::native_app::sample_library::folder_browser::view_contract::{
    CollectionRenameView, FOLDER_TREE_EDGE_CONTEXT_ROWS, FOLDER_TREE_OVERSCAN_ROWS,
    FOLDER_TREE_PROJECTED_VIEWPORT_ROWS, SampleCollectionView,
};
use crate::native_app::sample_library::folder_browser::{
    FolderBrowserState,
    model::{
        PLAYBACK_TYPE_FILTERS, PlaybackTypeFilter, RATING_FILTER_LEVELS, SourceEntry,
        VisibleFolder, playback_type_filter_label, rating_filter_label,
    },
};

pub(in crate::native_app) struct LibrarySidebarViewModel {
    pub(in crate::native_app) sidebar_width: f32,
    pub(in crate::native_app) metadata_panel_height: f32,
    pub(in crate::native_app) source_selector: SourceSelectorViewModel,
    pub(in crate::native_app) folder_tree: FolderTreeViewModel,
    pub(in crate::native_app) collections: CollectionsSectionViewModel,
    pub(in crate::native_app) filter: FilterSectionViewModel,
    pub(in crate::native_app) tag_editor: TagEditorViewModel,
}

pub(in crate::native_app) struct SourceSelectorViewModel {
    pub(in crate::native_app) rows: Vec<SourceRowViewModel>,
}

pub(in crate::native_app) struct SourceRowViewModel {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) scanning: bool,
}

pub(in crate::native_app) struct FolderTreeViewModel {
    pub(in crate::native_app) visible_folders: Vec<VisibleFolder>,
    pub(in crate::native_app) window: ui::VirtualListWindow,
    pub(in crate::native_app) selected_folder_status_label: String,
    pub(in crate::native_app) include_subfolders_available: bool,
    pub(in crate::native_app) include_subfolders: bool,
    pub(in crate::native_app) show_empty_folders: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

pub(in crate::native_app) struct CollectionsSectionViewModel {
    pub(in crate::native_app) rows: Vec<CollectionRowViewModel>,
    pub(in crate::native_app) panel_height: f32,
    pub(in crate::native_app) list_height: f32,
}

pub(in crate::native_app) struct CollectionRowViewModel {
    pub(in crate::native_app) collection: SampleCollectionView,
    pub(in crate::native_app) rename: Option<CollectionRenameView>,
}

pub(in crate::native_app) struct FilterSectionViewModel {
    pub(in crate::native_app) name_filter: String,
    pub(in crate::native_app) tag_filter: String,
    pub(in crate::native_app) playback_type_filters: Vec<PlaybackTypeFilterToggleViewModel>,
    pub(in crate::native_app) rating_filters: Vec<RatingFilterToggleViewModel>,
    pub(in crate::native_app) panel_height: f32,
}

pub(in crate::native_app) struct PlaybackTypeFilterToggleViewModel {
    pub(in crate::native_app) filter: PlaybackTypeFilter,
    pub(in crate::native_app) label: &'static str,
    pub(in crate::native_app) active: bool,
}

pub(in crate::native_app) struct RatingFilterToggleViewModel {
    pub(in crate::native_app) level: i8,
    pub(in crate::native_app) label: &'static str,
    pub(in crate::native_app) active: bool,
}

pub(in crate::native_app) struct TagEditorViewModel {
    pub(in crate::native_app) has_selected_file: bool,
    pub(in crate::native_app) draft: String,
    pub(in crate::native_app) tokens: Vec<String>,
    pub(in crate::native_app) pending_category_tag: Option<String>,
    pub(in crate::native_app) input_placeholder: String,
    pub(in crate::native_app) completion_suffix: Option<String>,
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) mixed_tags: Vec<String>,
    pub(in crate::native_app) display_categories: Vec<MetadataTagDisplayCategory>,
    pub(in crate::native_app) selected_tag: Option<String>,
}

impl LibrarySidebarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        let folder_browser = &state.library.folder_browser;
        Self {
            sidebar_width: state.ui.chrome.folder_panel.size(),
            metadata_panel_height: folder_browser.metadata_panel_height(),
            source_selector: SourceSelectorViewModel::from_folder_browser(folder_browser),
            folder_tree: FolderTreeViewModel::from_folder_browser(
                folder_browser,
                state.ui.chrome.help_tooltips_enabled,
            ),
            collections: CollectionsSectionViewModel::from_folder_browser(folder_browser),
            filter: FilterSectionViewModel::from_folder_browser(folder_browser),
            tag_editor: TagEditorViewModel::from_app_state(state),
        }
    }
}

impl SourceSelectorViewModel {
    pub(in crate::native_app) fn from_folder_browser(folder_browser: &FolderBrowserState) -> Self {
        let selected_source_id = folder_browser.selected_source_id();
        let rows = folder_browser
            .sources()
            .iter()
            .map(|source| SourceRowViewModel::from_source(source, selected_source_id))
            .collect();

        Self { rows }
    }
}

impl SourceRowViewModel {
    fn from_source(source: &SourceEntry, selected_source_id: &str) -> Self {
        Self {
            id: source.id.clone(),
            label: source.label.clone(),
            selected: selected_source_id == source.id,
            scanning: source.loading_task.is_some(),
        }
    }
}

impl FolderTreeViewModel {
    fn from_folder_browser(
        folder_browser: &FolderBrowserState,
        help_tooltips_enabled: bool,
    ) -> Self {
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
            selected_folder_status_label: folder_browser.selected_folder_status_label(),
            include_subfolders_available: folder_browser.folder_subtree_listing_available(),
            include_subfolders: folder_browser.folder_subtree_listing_enabled(),
            show_empty_folders: folder_browser.empty_folder_visibility_enabled(),
            help_tooltips_enabled,
        }
    }
}

impl CollectionsSectionViewModel {
    pub(in crate::native_app) fn from_folder_browser(folder_browser: &FolderBrowserState) -> Self {
        let rows = folder_browser
            .visible_collections()
            .into_iter()
            .map(|collection| {
                let rename = folder_browser.collection_rename_view(collection.collection);
                CollectionRowViewModel { collection, rename }
            })
            .collect::<Vec<_>>();
        Self {
            rows,
            panel_height: folder_browser.collections_panel_height(),
            list_height: folder_browser.collections_list_height(),
        }
    }
}

impl FilterSectionViewModel {
    pub(in crate::native_app) fn from_folder_browser(folder_browser: &FolderBrowserState) -> Self {
        Self {
            name_filter: folder_browser.name_filter().to_owned(),
            tag_filter: folder_browser.tag_filter().to_owned(),
            playback_type_filters: PLAYBACK_TYPE_FILTERS
                .into_iter()
                .map(|filter| PlaybackTypeFilterToggleViewModel {
                    filter,
                    label: playback_type_filter_label(filter),
                    active: folder_browser.playback_type_filter().contains(&filter),
                })
                .collect(),
            rating_filters: RATING_FILTER_LEVELS
                .into_iter()
                .map(|level| RatingFilterToggleViewModel {
                    level,
                    label: rating_filter_label(level),
                    active: folder_browser.rating_filter().contains(&level),
                })
                .collect(),
            panel_height: folder_browser.filter_panel_height(),
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
            tags: state.selected_metadata_tags_for_display(),
            mixed_tags: state.mixed_selected_metadata_tags_for_display(),
            display_categories: state.selected_metadata_tag_display_categories(),
            selected_tag: state.metadata.selected_tag.clone(),
        }
    }
}
