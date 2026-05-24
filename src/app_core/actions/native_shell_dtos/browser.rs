//! Browser projection DTOs for the Wavecrate native shell.

use super::RetainedVec;
use radiant::gui::badge;
use radiant::gui::selection;

mod row;

pub use self::row::{
    BrowserRowModel, BrowserRowProcessingState, PlaybackAgeBucket, PlaybackAgeFilterChip,
};

/// Tri-state pill state used by the browser metadata editor.
pub type BrowserTagState = selection::TriState;

/// One clickable tag pill projected into the browser metadata sidebar.
pub type BrowserTagPillModel = badge::SelectablePill<BrowserTagState>;

/// Browser-local metadata sidebar shown beside the sample list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserTagSidebarModel {
    /// Whether the panel should render in the current view.
    pub open: bool,
    /// Count of selected rows represented by the panel target set.
    pub selected_count: usize,
    /// Header line describing the current selection/focus context.
    pub header_label: String,
    /// Whether auto-rename is enabled for tag edits.
    pub primary_action_enabled: bool,
    /// Current tag search/create input value.
    pub input_value: String,
    /// Placeholder shown for the input when empty.
    pub input_placeholder: String,
    /// Whether the tag input currently owns text-editing focus.
    pub input_focused: bool,
    /// Caret position measured in Unicode scalar values from the start.
    pub input_caret: usize,
    /// Selected text range measured in Unicode scalar values, when any.
    pub input_selection: Option<(usize, usize)>,
    /// Mutually exclusive playback-mode pills.
    pub exclusive_pills: [BrowserTagPillModel; 2],
    /// Tags already applied to the represented target set.
    pub accepted_pills: Vec<BrowserTagPillModel>,
    /// Normal tag candidates from common usage or search.
    pub option_pills: Vec<BrowserTagPillModel>,
    /// Create-new tag candidate when the input does not match an existing option.
    pub create_pill: Option<BrowserTagPillModel>,
}

impl Default for BrowserTagSidebarModel {
    fn default() -> Self {
        Self {
            open: false,
            selected_count: 0,
            header_label: String::new(),
            primary_action_enabled: false,
            input_value: String::new(),
            input_placeholder: String::new(),
            input_focused: false,
            input_caret: 0,
            input_selection: None,
            exclusive_pills: [
                BrowserTagPillModel::default(),
                BrowserTagPillModel::default(),
            ],
            accepted_pills: Vec::new(),
            option_pills: Vec::new(),
            create_pill: None,
        }
    }
}

impl From<BrowserTagSidebarModel> for badge::PillEditorPanel<BrowserTagState> {
    fn from(value: BrowserTagSidebarModel) -> Self {
        Self {
            status: badge::PillEditorStatus {
                open: value.open,
                selected_count: value.selected_count,
                header_label: value.header_label,
                primary_action_enabled: value.primary_action_enabled,
            },
            input: badge::PillEditorInput {
                input_value: value.input_value,
                input_placeholder: value.input_placeholder,
                input_focused: value.input_focused,
                input_caret: value.input_caret,
                input_selection: value.input_selection,
            },
            choices: badge::PillEditorChoices {
                exclusive_pills: value.exclusive_pills,
                accepted_pills: value.accepted_pills,
                option_pills: value.option_pills,
                create_pill: value.create_pill,
            },
        }
    }
}

impl From<badge::PillEditorPanel<BrowserTagState>> for BrowserTagSidebarModel {
    fn from(value: badge::PillEditorPanel<BrowserTagState>) -> Self {
        Self {
            open: value.status.open,
            selected_count: value.status.selected_count,
            header_label: value.status.header_label,
            primary_action_enabled: value.status.primary_action_enabled,
            input_value: value.input.input_value,
            input_placeholder: value.input.input_placeholder,
            input_focused: value.input.input_focused,
            input_caret: value.input.input_caret,
            input_selection: value.input.input_selection,
            exclusive_pills: value.choices.exclusive_pills,
            accepted_pills: value.choices.accepted_pills,
            option_pills: value.choices.option_pills,
            create_pill: value.choices.create_pill,
        }
    }
}

/// Summary of browser/list state consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserPanelModel {
    /// Number of rows currently visible in the browser.
    pub visible_count: usize,
    /// Focused visible row index, if any.
    pub selected_visible_row: Option<usize>,
    /// Whether selection-driven browser autoscroll is currently enabled.
    pub autoscroll: bool,
    /// Requested top visible-row index for manual browser viewport scrolling.
    pub view_start_row: usize,
    /// Number of rows currently in multi-selection.
    pub selected_path_count: usize,
    /// Active browser search query.
    pub search_query: String,
    /// Active rating-filter chip states for levels `-3..=3`, plus `4` for locked keeps.
    pub active_rating_filters: [bool; 8],
    /// Active playback-age filter chip states ordered as `Never`, `Month`, `Week`.
    pub active_playback_age_filters: [bool; 3],
    /// Whether the browser is currently filtering down to only marked rows.
    pub marked_filter_active: bool,
    /// Whether the browser is currently filtering to tag-named rows.
    pub tag_named_filter_active: bool,
    /// Whether the tag-named filter is currently inverted.
    pub tag_named_filter_negated: bool,
    /// Sidebar metadata facets selected for browser filtering.
    pub sidebar_filters: crate::app_core::state::BrowserSidebarFilterState,
    /// Placeholder shown when the browser search query is empty.
    pub search_placeholder: Option<String>,
    /// Whether browser search/filter work is still running in the background.
    pub busy: bool,
    /// Whether the selected source is still hydrating before browser rows can project.
    pub source_loading: bool,
    /// Whether optimistic metadata writes are still pending background persistence.
    pub metadata_pending: bool,
    /// Whether file or folder mutations are still running in the background.
    pub file_op_pending: bool,
    /// Whether the browser is currently showing a similarity-filtered result set.
    pub similarity_filtered: bool,
    /// Whether browser duplicate cleanup mode is currently active.
    pub duplicate_cleanup_active: bool,
    /// Display label for the active browser sort mode.
    pub sort_label: Option<String>,
    /// Display label for the currently active browser tab.
    pub active_tab_label: Option<String>,
    /// Display label for the currently focused sample, when known.
    pub focused_sample_label: Option<String>,
    /// Metadata-tag editor sidebar projection scoped to the list tab.
    pub tag_sidebar: BrowserTagSidebarModel,
    /// Selection anchor in visible-row space.
    pub anchor_visible_row: Option<usize>,
    /// Visible rows rendered by the native browser panel.
    pub rows: RetainedVec<BrowserRowModel>,
}

impl BrowserPanelModel {
    /// Whether the generic derived-label filter is currently active.
    pub fn derived_label_filter_active(&self) -> bool {
        self.tag_named_filter_active
    }

    /// Whether the generic derived-label filter is currently inverted.
    pub fn derived_label_filter_negated(&self) -> bool {
        self.tag_named_filter_negated
    }

    /// Generic metadata-pill editor projected beside the content list.
    pub fn pill_editor(&self) -> &BrowserTagSidebarModel {
        &self.tag_sidebar
    }
}

/// Browser chrome copy used by the native shell toolbar and tab strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserChromeModel {
    /// Label for the list tab.
    pub samples_tab_label: String,
    /// Label for the browser item column.
    pub sample_column_label: String,
    /// Label for the map tab.
    pub map_tab_label: String,
    /// Label for the tag/pill editor action.
    pub tag_editor_label: String,
    /// Prefix label shown before active search queries.
    pub search_prefix_label: String,
    /// Placeholder label shown when no search query is active.
    pub search_placeholder: String,
    /// Status label shown when browser background work is idle.
    pub activity_ready_label: String,
    /// Status label shown when browser background work is running.
    pub activity_busy_label: String,
    /// Prefix label shown before active sort order labels.
    pub sort_prefix_label: String,
    /// Label describing the active sort order.
    pub sort_order_label: String,
    /// Label describing similarity mode in the map/header chrome.
    pub similarity_toggle_label: String,
    /// Footer/status label for total browser item counts.
    pub item_count_label: String,
}

impl Default for BrowserChromeModel {
    fn default() -> Self {
        Self {
            samples_tab_label: String::from("Samples"),
            sample_column_label: String::from("Sample"),
            map_tab_label: String::from("Similarity map"),
            tag_editor_label: String::from("Tags"),
            search_prefix_label: String::from("Search"),
            search_placeholder: String::from("Search samples (Ctrl+F)"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Filtering"),
            sort_prefix_label: String::from("Sort"),
            sort_order_label: String::from("List order"),
            similarity_toggle_label: String::from("points"),
            item_count_label: String::from("0 items"),
        }
    }
}

/// Browser action availability consumed by the native shell action strip.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserActionsModel {
    /// Whether rename can be started for the focused row.
    pub can_rename: bool,
    /// Whether delete can be applied to focused/selected rows.
    pub can_delete: bool,
    /// Whether tag actions can be applied to focused/selected rows.
    pub can_tag: bool,
    /// Whether the focused browser row can be normalized in place.
    pub can_normalize_focused_sample: bool,
    /// Whether the focused browser row can open the seamless loop-crossfade flow.
    pub can_loop_crossfade_focused_sample: bool,
    /// Whether sticky random navigation mode is currently enabled.
    pub random_navigation_enabled: bool,
    /// Whether browser duplicate cleanup mode is currently enabled.
    pub duplicate_cleanup_active: bool,
    /// Whether the browser-local tag sidebar is currently open.
    pub tag_sidebar_open: bool,
}

impl BrowserActionsModel {
    /// Whether generic browser pill edits can be applied.
    pub fn can_edit_pills(&self) -> bool {
        self.can_tag
    }

    /// Whether the generic browser pill editor is currently open.
    pub fn pill_editor_open(&self) -> bool {
        self.tag_sidebar_open
    }
}
