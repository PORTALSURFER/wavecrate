#[cfg(test)]
mod registry;

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
const SOURCES: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_5300);
const METADATA_TAGS: WidgetIdNamespace = WidgetIdNamespace::new(0x5743_0000_0000_5440);

pub(in crate::native_app) const WAVEFORM_VIEWPORT_STACK_ID: u64 = WAVEFORM.id(10);
pub(in crate::native_app) const WAVEFORM_SIGNAL_WIDGET_ID: u64 = WAVEFORM.id(11);
pub(in crate::native_app) const WAVEFORM_WIDGET_ID: u64 = WAVEFORM.id(12);
pub(in crate::native_app) const WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID: u64 = WAVEFORM.id(13);

pub(in crate::native_app) const FOLDER_TREE_LIST_ID: u64 = FOLDER_TREE.id(0);
pub(in crate::native_app) const FOLDER_TREE_INCLUDE_SUBFOLDERS_TOGGLE_ID: u64 = FOLDER_TREE.id(1);
pub(in crate::native_app) const FOLDER_TREE_SHOW_EMPTY_FOLDERS_TOGGLE_ID: u64 = FOLDER_TREE.id(2);
/// Scope for retained folder-row input identity.
pub(in crate::native_app) const RETAINED_FOLDER_TREE_ROW_INPUT_SCOPE: u64 = FOLDER_TREE.id(3);

pub(in crate::native_app) const SAMPLE_BROWSER_LIST_ID: u64 = SAMPLE_BROWSER.id(0);
/// Scope for retained sample-row input identity.
pub(in crate::native_app) const RETAINED_SAMPLE_ROW_INPUT_SCOPE: u64 = SAMPLE_BROWSER.id(1);
/// Scope for retained sample header-cell identity.
pub(in crate::native_app) const RETAINED_SAMPLE_HEADER_CELL_ID: u64 = SAMPLE_BROWSER_HEADER.id(1);
/// Automation-facing id for the random-navigation toggle.
pub(in crate::native_app) const AUTOMATION_SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID: u64 =
    SAMPLE_BROWSER_HEADER.id(3);
/// Automation-facing id for the similarity-weighting toggle.
pub(in crate::native_app) const AUTOMATION_SAMPLE_SIMILARITY_WEIGHTING_TOGGLE_ID: u64 =
    SAMPLE_BROWSER_HEADER.id(4);
/// Scope for automation-facing similarity-aspect toggle ids.
pub(in crate::native_app) const AUTOMATION_SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE: u64 =
    SAMPLE_BROWSER_HEADER.id(20);
/// Scope for automation-facing similarity-aspect weight slider ids.
pub(in crate::native_app) const AUTOMATION_SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE: u64 =
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
pub(in crate::native_app) const TOOLBAR_APPLY_EDIT_MARK_EDITS_ID: u64 = TOOLBAR.id(8);
pub(in crate::native_app) const TOOLBAR_SIMILAR_SECTIONS_ID: u64 = TOOLBAR.id(9);
pub(in crate::native_app) const TOOLBAR_METRONOME_ID: u64 = TOOLBAR.id(10);
pub(in crate::native_app) const TOOLBAR_ZERO_CROSSING_SNAP_ID: u64 = TOOLBAR.id(11);

#[cfg(test)]
pub(in crate::native_app) const FILTER_SECTION_NODE_ID: u64 = FOLDER_FILTERS.id(1);
pub(in crate::native_app) const NAME_FILTER_INPUT_ID: u64 = FOLDER_FILTERS.id(2);
pub(in crate::native_app) const TAG_FILTER_INPUT_ID: u64 = FOLDER_FILTERS.id(3);
pub(in crate::native_app) const FILTER_SECTION_SCROLL_NODE_ID: u64 = FOLDER_FILTERS.id(4);
pub(in crate::native_app) const FILTER_RESIZE_HEADER_ID: u64 = FOLDER_FILTERS.id(7);
/// Scope for automation-facing rating filter toggle ids.
pub(in crate::native_app) const AUTOMATION_RATING_FILTER_TOGGLE_SCOPE: u64 = FOLDER_FILTERS.id(20);
/// Scope for automation-facing playback-type filter toggle ids.
pub(in crate::native_app) const AUTOMATION_PLAYBACK_TYPE_FILTER_TOGGLE_SCOPE: u64 =
    FOLDER_FILTERS.id(21);

pub(in crate::native_app) const COLLECTIONS_SECTION_NODE_ID: u64 = COLLECTIONS.id(2);
pub(in crate::native_app) const COLLECTIONS_LIST_SCROLL_NODE_ID: u64 = COLLECTIONS.id(3);
#[cfg(test)]
pub(in crate::native_app) const EMPTY_COLLECTION_COUNT_NODE_ID: u64 = COLLECTIONS.id(4);
pub(in crate::native_app) const COLLECTIONS_RESIZE_HEADER_ID: u64 = COLLECTIONS.id(5);
/// Scope for retained collection-row input identity.
pub(in crate::native_app) const RETAINED_COLLECTION_ROW_INPUT_SCOPE: u64 = COLLECTIONS.id(1);

/// Automation-facing id for the add-source button.
pub(in crate::native_app) const AUTOMATION_SOURCE_ADD_BUTTON_ID: u64 = SOURCES.id(0);
/// Scope for retained source-row input identity.
pub(in crate::native_app) const RETAINED_SOURCE_ROW_INPUT_SCOPE: u64 = SOURCES.id(1);

pub(in crate::native_app) const METADATA_TAG_INPUT_ID: u64 = METADATA_TAGS.id(7);
#[cfg(test)]
pub(in crate::native_app) const METADATA_SIDEBAR_PANEL_ID: u64 = METADATA_TAGS.id(8);
#[cfg(test)]
pub(in crate::native_app) const METADATA_TAG_LIBRARY_TOGGLE_ID: u64 = METADATA_TAGS.id(9);
pub(in crate::native_app) const METADATA_RESIZE_HEADER_ID: u64 = METADATA_TAGS.id(10);
pub(in crate::native_app) const METADATA_CATEGORY_ROW_INPUT_SCOPE: u64 = METADATA_TAGS.id(11);
