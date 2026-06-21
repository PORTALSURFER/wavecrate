#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
struct WidgetIdNamespace {
    base: u64,
}

#[cfg(test)]
const WIDGET_ID_NAMESPACE_SIZE: u64 = 1_000;

impl WidgetIdNamespace {
    const fn new(base: u64) -> Self {
        Self { base }
    }

    const fn id(self, offset: u16) -> u64 {
        self.base + offset as u64
    }

    #[cfg(test)]
    const fn contains(self, value: u64) -> bool {
        value >= self.base && value < self.base + WIDGET_ID_NAMESPACE_SIZE
    }
}

const WAVEFORM: WidgetIdNamespace = WidgetIdNamespace::new(0);
const FOLDER_TREE: WidgetIdNamespace = WidgetIdNamespace::new(29_000);
const SAMPLE_BROWSER: WidgetIdNamespace = WidgetIdNamespace::new(30_000);
const AUDIO_SETTINGS: WidgetIdNamespace = WidgetIdNamespace::new(31_000);
const TRANSACTION_HISTORY: WidgetIdNamespace = WidgetIdNamespace::new(31_200);
const TOOLBAR: WidgetIdNamespace = WidgetIdNamespace::new(32_100);
const FOLDER_FILTERS: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_4600);
const SAMPLE_BROWSER_HEADER: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_4800);
const COLLECTIONS: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_4c00);
const METADATA_TAGS: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_5440);

pub(in crate::native_app) const WAVEFORM_VIEWPORT_STACK_ID: u64 = WAVEFORM.id(10);
pub(in crate::native_app) const WAVEFORM_SIGNAL_WIDGET_ID: u64 = WAVEFORM.id(11);
pub(in crate::native_app) const WAVEFORM_WIDGET_ID: u64 = WAVEFORM.id(12);

pub(in crate::native_app) const FOLDER_TREE_LIST_ID: u64 = FOLDER_TREE.id(0);
pub(in crate::native_app) const FOLDER_TREE_INCLUDE_SUBFOLDERS_TOGGLE_ID: u64 = FOLDER_TREE.id(1);
pub(in crate::native_app) const FOLDER_TREE_SHOW_EMPTY_FOLDERS_TOGGLE_ID: u64 = FOLDER_TREE.id(2);

pub(in crate::native_app) const SAMPLE_BROWSER_LIST_ID: u64 = SAMPLE_BROWSER.id(0);
pub(in crate::native_app) const SAMPLE_HEADER_SORT_DRAG_ID: u64 = SAMPLE_BROWSER_HEADER.id(1);
pub(in crate::native_app) const SAMPLE_HEADER_RESIZE_ID: u64 = SAMPLE_BROWSER_HEADER.id(2);
pub(in crate::native_app) const SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID: u64 =
    SAMPLE_BROWSER_HEADER.id(3);
pub(in crate::native_app) const SAMPLE_SIMILARITY_WEIGHTING_TOGGLE_ID: u64 =
    SAMPLE_BROWSER_HEADER.id(4);
pub(in crate::native_app) const SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE: u64 =
    SAMPLE_BROWSER_HEADER.id(20);
pub(in crate::native_app) const SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE: u64 =
    SAMPLE_BROWSER_HEADER.id(21);

pub(in crate::native_app) const VOLUME_SLIDER_ID: u64 = AUDIO_SETTINGS.id(0);
pub(in crate::native_app) const HELP_TOOLTIPS_BUTTON_ID: u64 = AUDIO_SETTINGS.id(90);
pub(in crate::native_app) const AUDIO_ENGINE_PILL_ID: u64 = AUDIO_SETTINGS.id(100);
pub(in crate::native_app) const GENERAL_SETTINGS_BUTTON_ID: u64 = AUDIO_SETTINGS.id(110);

pub(in crate::native_app) const TRANSACTION_LIST_MODAL_ID: u64 = TRANSACTION_HISTORY.id(0);

pub(in crate::native_app) const TOOLBAR_FOCUS_LOADED_ID: u64 = TOOLBAR.id(0);
pub(in crate::native_app) const TOOLBAR_LOOP_ID: u64 = TOOLBAR.id(1);
pub(in crate::native_app) const TOOLBAR_PLAY_ID: u64 = TOOLBAR.id(2);
pub(in crate::native_app) const TOOLBAR_STOP_ID: u64 = TOOLBAR.id(3);
pub(in crate::native_app) const TOOLBAR_RANDOM_ID: u64 = TOOLBAR.id(4);
pub(in crate::native_app) const TOOLBAR_BEAT_GUIDES_ID: u64 = TOOLBAR.id(5);
pub(in crate::native_app) const TOOLBAR_BEAT_GUIDE_DECREMENT_ID: u64 = TOOLBAR.id(6);
pub(in crate::native_app) const TOOLBAR_BEAT_GUIDE_INCREMENT_ID: u64 = TOOLBAR.id(7);

#[cfg(test)]
pub(in crate::native_app) const FILTER_SECTION_NODE_ID: u64 = FOLDER_FILTERS.id(1);
pub(in crate::native_app) const NAME_FILTER_INPUT_ID: u64 = FOLDER_FILTERS.id(2);
pub(in crate::native_app) const TAG_FILTER_INPUT_ID: u64 = FOLDER_FILTERS.id(3);
pub(in crate::native_app) const FILTER_SECTION_SCROLL_NODE_ID: u64 = FOLDER_FILTERS.id(4);
pub(in crate::native_app) const NAME_FILTER_CLEAR_BUTTON_ID: u64 = FOLDER_FILTERS.id(5);
pub(in crate::native_app) const TAG_FILTER_CLEAR_BUTTON_ID: u64 = FOLDER_FILTERS.id(6);
pub(in crate::native_app) const FILTER_RESIZE_HEADER_ID: u64 = FOLDER_FILTERS.id(7);
pub(in crate::native_app) const RATING_FILTER_TOGGLE_SCOPE: u64 = FOLDER_FILTERS.id(20);

pub(in crate::native_app) const COLLECTIONS_SECTION_NODE_ID: u64 = COLLECTIONS.id(2);
pub(in crate::native_app) const COLLECTIONS_LIST_SCROLL_NODE_ID: u64 = COLLECTIONS.id(3);
#[cfg(test)]
pub(in crate::native_app) const EMPTY_COLLECTION_COUNT_NODE_ID: u64 = COLLECTIONS.id(4);
pub(in crate::native_app) const COLLECTIONS_RESIZE_HEADER_ID: u64 = COLLECTIONS.id(5);

pub(in crate::native_app) const METADATA_TAG_INPUT_ID: u64 = METADATA_TAGS.id(7);
#[cfg(test)]
pub(in crate::native_app) const METADATA_SIDEBAR_PANEL_ID: u64 = METADATA_TAGS.id(8);
#[cfg(test)]
pub(in crate::native_app) const METADATA_TAG_LIBRARY_TOGGLE_ID: u64 = METADATA_TAGS.id(9);
pub(in crate::native_app) const METADATA_RESIZE_HEADER_ID: u64 = METADATA_TAGS.id(10);

#[cfg(test)]
#[derive(Clone, Copy)]
struct RegisteredWidgetId {
    owner: WidgetIdOwner,
    constant: &'static str,
    stable_name: &'static str,
    value: u64,
}

#[cfg(test)]
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
    MetadataTags,
}

#[cfg(test)]
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
            Self::MetadataTags => METADATA_TAGS,
        }
    }
}

#[cfg(test)]
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

#[cfg(test)]
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
        SAMPLE_HEADER_SORT_DRAG_ID,
        "sample_browser.header_sort_drag"
    ),
    registered_widget_id!(
        SampleBrowserHeader,
        SAMPLE_HEADER_RESIZE_ID,
        "sample_browser.header_resize"
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
        NAME_FILTER_CLEAR_BUTTON_ID,
        "folder_filters.name_clear_button"
    ),
    registered_widget_id!(
        FolderFilters,
        TAG_FILTER_CLEAR_BUTTON_ID,
        "folder_filters.tag_clear_button"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_native_app_widget_ids_are_unique() {
        let mut values = BTreeMap::new();
        let mut constants = BTreeSet::new();
        let mut stable_names = BTreeSet::new();

        for id in REGISTERED_WIDGET_IDS {
            assert_ne!(
                id.value, 0,
                "{} must not use the zero widget id",
                id.constant
            );
            assert!(
                constants.insert(id.constant),
                "{} is registered more than once",
                id.constant
            );
            assert!(
                stable_names.insert(id.stable_name),
                "{} is registered more than once",
                id.stable_name
            );
            if let Some(previous) = values.insert(id.value, id) {
                panic!(
                    "duplicate native app widget id {:#x}: {} and {}",
                    id.value, previous.constant, id.constant
                );
            }
        }
    }

    #[test]
    fn registered_native_app_widget_ids_stay_inside_owner_namespaces() {
        for id in REGISTERED_WIDGET_IDS {
            assert!(
                id.owner.namespace().contains(id.value),
                "{} ({}) must stay inside the {:?} widget id namespace",
                id.constant,
                id.stable_name,
                id.owner
            );
        }
    }
}
