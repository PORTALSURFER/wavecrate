use super::*;

mod tests;

#[derive(Clone, Copy)]
struct RegisteredWidgetId {
    owner: WidgetIdOwner,
    constant: &'static str,
    stable_name: &'static str,
    value: u64,
}

#[derive(Clone, Copy, Debug)]
enum WidgetIdOwner {
    Waveform,
    FolderTree,
    SampleBrowser,
    SampleBrowserHeader,
    AudioSettings,
    TransactionHistory,
    Toolbar,
    FolderFilters,
    Collections,
    Sources,
    MetadataTags,
}

impl WidgetIdOwner {
    const fn namespace(self) -> WidgetIdNamespace {
        match self {
            Self::Waveform => WAVEFORM,
            Self::FolderTree => FOLDER_TREE,
            Self::SampleBrowser => SAMPLE_BROWSER,
            Self::SampleBrowserHeader => SAMPLE_BROWSER_HEADER,
            Self::AudioSettings => AUDIO_SETTINGS,
            Self::TransactionHistory => TRANSACTION_HISTORY,
            Self::Toolbar => TOOLBAR,
            Self::FolderFilters => FOLDER_FILTERS,
            Self::Collections => COLLECTIONS,
            Self::Sources => SOURCES,
            Self::MetadataTags => METADATA_TAGS,
        }
    }
}

macro_rules! registered_widget_id {
    ($owner:ident, $constant:ident, $stable_name:literal) => {
        RegisteredWidgetId {
            owner: WidgetIdOwner::$owner,
            constant: stringify!($constant),
            stable_name: $stable_name,
            value: $constant,
        }
    };
}

const REGISTERED_WIDGET_IDS: &[RegisteredWidgetId] = &[
    registered_widget_id!(
        Waveform,
        WAVEFORM_VIEWPORT_STACK_ID,
        "waveform.viewport_stack"
    ),
    registered_widget_id!(
        Waveform,
        WAVEFORM_SIGNAL_WIDGET_ID,
        "waveform.signal_surface"
    ),
    registered_widget_id!(Waveform, WAVEFORM_WIDGET_ID, "waveform.interaction_widget"),
    registered_widget_id!(
        Waveform,
        WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID,
        "waveform.loaded_sample_drag_handle"
    ),
    registered_widget_id!(FolderTree, FOLDER_TREE_LIST_ID, "folder_tree.list"),
    registered_widget_id!(
        FolderTree,
        FOLDER_TREE_INCLUDE_SUBFOLDERS_TOGGLE_ID,
        "folder_tree.include_subfolders_toggle"
    ),
    registered_widget_id!(
        FolderTree,
        FOLDER_TREE_SHOW_EMPTY_FOLDERS_TOGGLE_ID,
        "folder_tree.show_empty_folders_toggle"
    ),
    registered_widget_id!(SampleBrowser, SAMPLE_BROWSER_LIST_ID, "sample_browser.list"),
    registered_widget_id!(
        SampleBrowserHeader,
        SAMPLE_HEADER_CELL_ID,
        "sample_browser.header_cell"
    ),
    registered_widget_id!(
        SampleBrowserHeader,
        SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID,
        "sample_browser.random_navigation_toggle"
    ),
    registered_widget_id!(
        SampleBrowserHeader,
        SAMPLE_SIMILARITY_WEIGHTING_TOGGLE_ID,
        "sample_browser.similarity_weighting_toggle"
    ),
    registered_widget_id!(
        AudioSettings,
        VOLUME_SLIDER_ID,
        "audio_settings.volume_slider"
    ),
    registered_widget_id!(
        AudioSettings,
        HELP_TOOLTIPS_BUTTON_ID,
        "audio_settings.help_tooltips_button"
    ),
    registered_widget_id!(
        AudioSettings,
        AUDIO_ENGINE_PILL_ID,
        "audio_settings.engine_pill"
    ),
    registered_widget_id!(
        AudioSettings,
        GENERAL_SETTINGS_BUTTON_ID,
        "audio_settings.general_settings_button"
    ),
    registered_widget_id!(
        TransactionHistory,
        TRANSACTION_LIST_MODAL_ID,
        "transaction_history.list_modal"
    ),
    registered_widget_id!(Toolbar, TOOLBAR_FOCUS_LOADED_ID, "toolbar.focus_loaded"),
    registered_widget_id!(Toolbar, TOOLBAR_LOOP_ID, "toolbar.loop"),
    registered_widget_id!(Toolbar, TOOLBAR_PLAY_ID, "toolbar.play"),
    registered_widget_id!(Toolbar, TOOLBAR_STOP_ID, "toolbar.stop"),
    registered_widget_id!(Toolbar, TOOLBAR_RANDOM_ID, "toolbar.random"),
    registered_widget_id!(Toolbar, TOOLBAR_BEAT_GUIDES_ID, "toolbar.beat_guides"),
    registered_widget_id!(
        Toolbar,
        TOOLBAR_BEAT_GUIDE_DECREMENT_ID,
        "toolbar.beat_guide_decrement"
    ),
    registered_widget_id!(
        Toolbar,
        TOOLBAR_BEAT_GUIDE_INCREMENT_ID,
        "toolbar.beat_guide_increment"
    ),
    registered_widget_id!(
        Toolbar,
        TOOLBAR_APPLY_EDIT_MARK_EDITS_ID,
        "toolbar.apply_edit_mark_edits"
    ),
    registered_widget_id!(
        Toolbar,
        TOOLBAR_SIMILAR_SECTIONS_ID,
        "toolbar.similar_sections"
    ),
    registered_widget_id!(
        FolderFilters,
        FILTER_SECTION_NODE_ID,
        "folder_filters.section"
    ),
    registered_widget_id!(
        FolderFilters,
        NAME_FILTER_INPUT_ID,
        "folder_filters.name_input"
    ),
    registered_widget_id!(
        FolderFilters,
        TAG_FILTER_INPUT_ID,
        "folder_filters.tag_input"
    ),
    registered_widget_id!(
        FolderFilters,
        FILTER_SECTION_SCROLL_NODE_ID,
        "folder_filters.scroll"
    ),
    registered_widget_id!(
        FolderFilters,
        FILTER_RESIZE_HEADER_ID,
        "folder_filters.resize_header"
    ),
    registered_widget_id!(
        Collections,
        COLLECTIONS_SECTION_NODE_ID,
        "collections.section"
    ),
    registered_widget_id!(
        Collections,
        COLLECTIONS_LIST_SCROLL_NODE_ID,
        "collections.list_scroll"
    ),
    registered_widget_id!(
        Collections,
        EMPTY_COLLECTION_COUNT_NODE_ID,
        "collections.empty_count"
    ),
    registered_widget_id!(
        Collections,
        COLLECTIONS_RESIZE_HEADER_ID,
        "collections.resize_header"
    ),
    registered_widget_id!(Sources, SOURCE_ADD_BUTTON_ID, "sources.add_button"),
    registered_widget_id!(MetadataTags, METADATA_TAG_INPUT_ID, "metadata_tags.input"),
    registered_widget_id!(
        MetadataTags,
        METADATA_SIDEBAR_PANEL_ID,
        "metadata_tags.sidebar_panel"
    ),
    registered_widget_id!(
        MetadataTags,
        METADATA_TAG_LIBRARY_TOGGLE_ID,
        "metadata_tags.library_toggle"
    ),
    registered_widget_id!(
        MetadataTags,
        METADATA_RESIZE_HEADER_ID,
        "metadata_tags.resize_header"
    ),
];
