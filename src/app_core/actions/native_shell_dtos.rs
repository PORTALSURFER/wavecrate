//! Sempal-owned native shell projection DTOs.
//!
//! These models describe Sempal application state as projected for the current
//! native shell. Radiant still consumes a compatibility copy at the runtime
//! boundary, so this module also provides field-for-field adapters that preserve
//! the legacy shell snapshot contract without making Radiant the owner of the
//! Sempal projection types.

use radiant::compat::legacy_shell as compat;
use radiant::gui::automation;
use radiant::gui::badge;
use radiant::gui::chrome;
use radiant::gui::feedback;
use radiant::gui::form;
use radiant::gui::frame;
use radiant::gui::invalidation;
use radiant::gui::list;
use radiant::gui::panel;
use radiant::gui::range;
use radiant::gui::retained;
use radiant::gui::selection;
use radiant::gui::types::ImageRgba;
use radiant::gui::visualization;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

/// Shared storage used by retained app-model snapshots.
pub type RetainedVec<T> = retained::RetainedVec<T>;

/// Stable semantic identifier for one automation node in the native shell tree.
pub type AutomationNodeId = automation::AutomationNodeId;

/// Quantized window-space bounds for one automation node.
pub type AutomationBounds = automation::AutomationBounds;

/// Frame-level feedback from renderer to host bridge.
pub type FrameBuildResult = frame::FrameBuildResult;

/// Normalized interval with deterministic milli, micro, and nano projections.
pub type NormalizedRangeModel = range::NormalizedRange;

/// Structured footer status content for left/center/right status segments.
pub type StatusBarModel = chrome::StatusSegments;

/// Progress overlay state projected into the native shell.
pub type ProgressOverlayModel = feedback::ProgressOverlay;

/// Drag/drop overlay content for native-shell feedback.
pub type DragOverlayModel = feedback::DragOverlay;

/// Render data for one triage/browser column.
pub type ColumnModel = list::ColumnSummary;

/// Render data for one folder row shown in the sidebar folder tree.
pub type FolderRowKind = list::EditableRowKind;

/// Render data for one folder row shown in the sidebar folder tree.
pub type FolderRowModel = list::EditableTreeRow;

/// Native folder-action availability consumed by sidebar action surfaces.
pub type FolderActionsModel = list::EditableTreeActions;

/// Stable identifier for one side of the split folder pane surface.
pub type FolderPaneIdModel = panel::SplitPaneSlot;

/// Projected data for one fixed folder pane shown in the sidebar.
pub type FolderPaneModel = panel::SplitPaneTreePanel<FolderRowModel>;

/// Render data for one source row shown in the sidebar.
pub type SourceRowModel = panel::SplitPaneAssignedRow;

/// Transient browser row processing states for batch file operations.
pub type BrowserRowProcessingState = list::RowProcessingState;

/// Summary of one browser/list row consumed by the native shell.
pub type BrowserRowModel = list::ContentListRow;

/// Tri-state pill state used by the browser metadata editor.
pub type BrowserTagState = selection::TriState;

/// One clickable tag pill projected into the browser metadata sidebar.
pub type BrowserTagPillModel = badge::SelectablePill<BrowserTagState>;

/// Browser-local metadata sidebar shown beside the sample list.
pub type BrowserTagSidebarModel = badge::PillEditorPanel<BrowserTagState>;

/// Render mode label for the map panel.
pub type MapRenderModeModel = visualization::PointRenderMode;

/// Summary of map state consumed by the native shell map tab.
pub type MapPanelModel = visualization::SpatialPanel;

/// Channel-view mode used by waveform rendering.
pub type WaveformChannelViewModel = visualization::ChannelViewMode;

/// One detected waveform slice preview exposed to the native shell.
pub type WaveformSlicePreviewModel = visualization::TimelineMarkerPreview;

/// One-shot waveform feedback event tokens exposed to the native shell.
pub type WaveformFeedbackEventsModel = visualization::TimelineFeedbackEvents;

/// Waveform guide/repeat/label presentation state exposed to the native shell.
pub type WaveformPresentationModel = visualization::TimelinePresentationState;

/// Render data for one point shown in the native map canvas.
pub type MapPointModel = visualization::SpatialPoint;

/// Update-check status projected into the native shell.
pub type UpdateStatusModel = feedback::UpdateStatus;

/// Update panel state used by native top-bar actions.
pub type UpdatePanelModel = feedback::UpdatePanel;

/// Modal confirmation prompt projected into the native shell.
pub type ConfirmPromptModel = feedback::ConfirmPrompt<ConfirmPromptKind>;

/// Health state of the compact audio-engine status chip.
pub type AudioEngineChipStateModel = feedback::HealthState;

/// Delete-recovery status for staged folder delete recovery in the sidebar.
pub type FolderRecoveryModel = feedback::RecoverySummary;

/// One selectable item shown inside an audio picker.
pub type AudioOptionItemModel = form::OptionItem<AudioOptionValueModel>;

/// Overview row shown for one audio field inside the options panel.
pub type AudioFieldModel = form::SummaryField;

/// Browser playback-age filter chips shown in the native toolbar.
pub type PlaybackAgeFilterChip = list::RecencyFilterChip;

/// Generic preference/settings panel state used by native overlay projections.
pub type PreferencePanelStateModel<const TOGGLES: usize> = form::PreferencePanelState<TOGGLES>;

// Sempal-owned GUI automation snapshot DTOs.

/// Semantic role describing how an automation node behaves in the GUI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRole {
    /// Synthetic root of the automation snapshot tree.
    Root,
    /// Grouping container such as a panel or composite section.
    Group,
    /// Major panel surface.
    Panel,
    /// Toolbar or action strip.
    Toolbar,
    /// Tab-strip container.
    TabList,
    /// Toggleable tab node.
    Tab,
    /// Clickable button.
    Button,
    /// Search or text-entry field.
    SearchField,
    /// Slider or continuous meter interaction surface.
    Slider,
    /// Row in a list or table.
    Row,
    /// Table or row-hosting list surface.
    Table,
    /// Waveform interaction canvas.
    WaveformRegion,
    /// Map interaction canvas.
    MapCanvas,
    /// Focusable point inside the map canvas.
    MapPoint,
    /// Status/readout region.
    Readout,
    /// Dialog or modal container.
    Dialog,
}

/// One node in the GUI automation tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationNodeSnapshot {
    /// Stable semantic identifier for this node.
    pub id: AutomationNodeId,
    /// Behavioral role for this node.
    pub role: AutomationRole,
    /// Optional human-readable label shown by the GUI.
    pub label: Option<String>,
    /// Quantized window-space bounds.
    pub bounds: AutomationBounds,
    /// Optional current value or summary text.
    pub value: Option<String>,
    /// Whether the node is currently enabled.
    pub enabled: bool,
    /// Whether the node is currently selected or active.
    pub selected: bool,
    /// Stable action identifiers that this node can trigger.
    pub available_actions: Vec<String>,
    /// Additional deterministic metadata for AI/test consumers.
    pub metadata: BTreeMap<String, String>,
    /// Child nodes in semantic tree order.
    pub children: Vec<AutomationNodeSnapshot>,
}

/// Full deterministic automation snapshot emitted for one GUI frame/state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuiAutomationSnapshot {
    /// Schema version for forward-compatible artifact readers.
    pub schema_version: u32,
    /// Quantized viewport width for the captured shell layout.
    pub viewport_width: u32,
    /// Quantized viewport height for the captured shell layout.
    pub viewport_height: u32,
    /// Root semantic automation node.
    pub root: AutomationNodeSnapshot,
}

// Sempal-owned retained-render segment invalidation DTOs.

/// Bitmask describing which projection segments changed during the last model pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtySegments {
    mask: invalidation::RetainedSegmentMask<0x00ff, 0x003f, 0x00c0>,
}

impl DirtySegments {
    /// Status-bar content segment.
    pub const STATUS_BAR: u16 = 1 << 0;
    /// Browser metadata/chrome segment.
    pub const BROWSER_FRAME: u16 = 1 << 1;
    /// Browser row-window segment.
    pub const BROWSER_ROWS_WINDOW: u16 = 1 << 2;
    /// Map-panel segment.
    pub const MAP_PANEL: u16 = 1 << 3;
    /// Waveform panel/chrome segment.
    pub const WAVEFORM_OVERLAY: u16 = 1 << 4;
    /// Static content that is outside explicit segment buckets.
    pub const GLOBAL_STATIC: u16 = 1 << 5;
    /// State-overlay model fields.
    pub const STATE_OVERLAY: u16 = 1 << 6;
    /// Motion-overlay model fields.
    pub const MOTION_OVERLAY: u16 = 1 << 7;

    /// Return an empty segment mask.
    pub const fn empty() -> Self {
        Self {
            mask: invalidation::RetainedSegmentMask::empty(),
        }
    }

    /// Return a full segment mask.
    pub const fn all() -> Self {
        Self {
            mask: invalidation::RetainedSegmentMask::all(),
        }
    }

    /// Construct a segment mask from raw bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self {
            mask: invalidation::RetainedSegmentMask::from_bits(bits),
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.mask.bits()
    }

    /// Return `true` when the mask contains no segments.
    pub const fn is_empty(self) -> bool {
        self.mask.is_empty()
    }

    /// Return `true` when any static segment requires rebuild.
    pub const fn requires_static_rebuild(self) -> bool {
        self.mask.requires_static_rebuild()
    }

    /// Return `true` when any overlay segment requires rebuild.
    pub const fn requires_overlay_rebuild(self) -> bool {
        self.mask.requires_overlay_rebuild()
    }

    /// Insert one or more segment bits into this mask.
    pub fn insert(&mut self, bits: u16) {
        self.mask.insert(bits);
    }
}

/// Monotonic revision counters for static projection segments.
///
/// Bridges bump the counters for segments whose projected model slices changed on
/// the most recent `pull_model`. Runtimes use these revisions in retained-scene
/// cache keys to avoid expensive segment hashing on every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SegmentRevisions {
    /// Status-bar projection revision.
    pub status_bar: u64,
    /// Browser metadata/chrome projection revision.
    pub browser_frame: u64,
    /// Browser visible-row window projection revision.
    pub browser_rows_window: u64,
    /// Map-panel projection revision.
    pub map_panel: u64,
    /// Waveform panel/chrome projection revision.
    pub waveform_overlay: u64,
    /// Global static fields projection revision.
    pub global_static: u64,
}

impl SegmentRevisions {
    /// Return these named compatibility revisions as a generic retained segment array.
    pub const fn retained_revisions(self) -> invalidation::RetainedSegmentRevisions<6> {
        invalidation::RetainedSegmentRevisions::new([
            self.status_bar,
            self.browser_frame,
            self.browser_rows_window,
            self.map_panel,
            self.waveform_overlay,
            self.global_static,
        ])
    }

    /// Return whether any static-segment revision is non-zero.
    pub fn has_static_revisions(self) -> bool {
        self.retained_revisions().has_revisions()
    }

    /// Bump revisions for the static segments flagged in `dirty_segments`.
    pub fn bump_for_dirty_segments(&mut self, dirty_segments: DirtySegments) {
        let bits = dirty_segments.bits();
        let mut revisions = self.retained_revisions();
        revisions.bump_for_bits(
            bits,
            [
                DirtySegments::STATUS_BAR,
                DirtySegments::BROWSER_FRAME,
                DirtySegments::BROWSER_ROWS_WINDOW,
                DirtySegments::MAP_PANEL,
                DirtySegments::WAVEFORM_OVERLAY,
                DirtySegments::GLOBAL_STATIC,
            ],
        );
        let [
            status_bar,
            browser_frame,
            browser_rows_window,
            map_panel,
            waveform_overlay,
            global_static,
        ] = revisions.revisions;
        self.status_bar = status_bar;
        self.browser_frame = browser_frame;
        self.browser_rows_window = browser_rows_window;
        self.map_panel = map_panel;
        self.waveform_overlay = waveform_overlay;
        self.global_static = global_static;
    }
}

// Sempal-owned motion-only projection DTOs.

/// Motion-sensitive slice of the app model used for incremental overlay rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeMotionModel {
    /// Transport animation state used by motion overlays.
    pub transport_running: bool,
    /// Whether map mode is active for tab overlay tinting.
    pub map_active: bool,
    /// Active browser rating-filter chip states for levels `-3..=3`, plus `4` for locked keeps.
    pub active_rating_filters: [bool; 8],
    /// Active browser playback-age filter chip states ordered as `Never`, `Month`, `Week`.
    pub active_playback_age_filters: [bool; 3],
    /// Whether the browser is currently filtering down to session-marked rows.
    pub marked_filter_active: bool,
    /// Waveform selected playback window with milli and micro precision.
    pub waveform_selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub waveform_slices: Vec<WaveformSlicePreviewModel>,
    /// One-shot token incremented when a waveform-selection export is queued.
    pub waveform_selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    pub waveform_selection_export_failure_flash_nonce: u64,
    /// One-shot token incremented when preview edit fades are committed.
    pub waveform_edit_selection_apply_flash_nonce: u64,
    /// Waveform edit-selection window with milli and micro precision.
    pub waveform_edit_selection_milli: Option<NormalizedRangeModel>,
    /// Waveform edit fade-in end handle in normalized milliseconds.
    pub waveform_edit_fade_in_end_milli: Option<u16>,
    /// Waveform edit fade-in end handle in normalized micro-units.
    pub waveform_edit_fade_in_end_micros: Option<u32>,
    /// Waveform edit fade-in mute-start handle in normalized milliseconds.
    pub waveform_edit_fade_in_mute_start_milli: Option<u16>,
    /// Waveform edit fade-in mute-start handle in normalized micro-units.
    pub waveform_edit_fade_in_mute_start_micros: Option<u32>,
    /// Waveform edit fade-in curve tension in normalized milliseconds.
    pub waveform_edit_fade_in_curve_milli: Option<u16>,
    /// Waveform edit fade-out start handle in normalized milliseconds.
    pub waveform_edit_fade_out_start_milli: Option<u16>,
    /// Waveform edit fade-out start handle in normalized micro-units.
    pub waveform_edit_fade_out_start_micros: Option<u32>,
    /// Waveform edit fade-out mute-end handle in normalized milliseconds.
    pub waveform_edit_fade_out_mute_end_milli: Option<u16>,
    /// Waveform edit fade-out mute-end handle in normalized micro-units.
    pub waveform_edit_fade_out_mute_end_micros: Option<u32>,
    /// Waveform edit fade-out curve tension in normalized milliseconds.
    pub waveform_edit_fade_out_curve_milli: Option<u16>,
    /// Whether loop playback is enabled for the active waveform selection.
    pub waveform_loop_enabled: bool,
    /// Whether loop playback is currently locked against sample-driven updates.
    pub waveform_loop_lock_enabled: bool,
    /// Waveform cursor position in normalized milliseconds.
    pub waveform_cursor_milli: Option<u16>,
    /// Waveform playhead position in normalized milliseconds.
    pub waveform_playhead_milli: Option<u16>,
    /// Waveform playhead position in normalized micro-units (`0..=1_000_000`).
    pub waveform_playhead_micros: Option<u32>,
    /// Current waveform view start in normalized milliseconds.
    pub waveform_view_start_milli: u16,
    /// Current waveform view end in normalized milliseconds.
    pub waveform_view_end_milli: u16,
    /// Current waveform view start in normalized micro-units (`0..=1_000_000`).
    pub waveform_view_start_micros: u32,
    /// Current waveform view end in normalized micro-units (`0..=1_000_000`).
    pub waveform_view_end_micros: u32,
    /// Current waveform view start in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Motion overlays use nanosecond bounds so rendered selection edges and
    /// playhead markers stay aligned with deep-zoom pointer geometry.
    pub waveform_view_start_nanos: u32,
    /// Current waveform view end in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Motion overlays use nanosecond bounds so rendered selection edges and
    /// playhead markers stay aligned with deep-zoom pointer geometry.
    pub waveform_view_end_nanos: u32,
    /// Human-readable tempo metadata.
    pub waveform_tempo_label: Option<String>,
    /// Human-readable zoom metadata.
    pub waveform_zoom_label: Option<String>,
    /// Loaded waveform label shown in the waveform overlay header.
    pub waveform_loaded_label: Option<String>,
    /// Whether the waveform plot is currently waiting for a new sample to load.
    pub waveform_loading: bool,
    /// Stable image signature for detecting waveform image updates during motion-only frames.
    pub waveform_image_signature: Option<u64>,
    /// Transport hint rendered with waveform metadata.
    pub waveform_transport_hint: String,
    /// Whether compare-anchor replay is currently available.
    pub waveform_compare_anchor_available: bool,
    /// Label for the stored compare anchor, when available.
    pub waveform_compare_anchor_label: Option<String>,
    /// Current waveform channel-view mode.
    pub waveform_channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub waveform_normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled.
    pub waveform_bpm_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub waveform_relative_bpm_grid_enabled: bool,
    /// Whether transient snapping is enabled.
    pub waveform_transient_snap_enabled: bool,
    /// Whether transient markers are visible.
    pub waveform_transient_markers_enabled: bool,
    /// Whether slice mode is active.
    pub waveform_slice_mode_enabled: bool,
    /// Whether exact-duplicate cleanup can be applied from the waveform toolbar.
    pub waveform_exact_duplicate_cleanup_available: bool,
    /// Right-aligned status-bar text rendered in the motion overlay.
    pub status_right: String,
}

impl NativeMotionModel {
    /// Build a motion model from a full application model snapshot.
    pub fn from_app_model(model: &AppModel) -> Self {
        let viewport = model.waveform.viewport();
        let transport = model.waveform.transport();
        let edit_preview = model.waveform.edit_preview();
        let feedback_events = model.waveform.feedback_events();
        let presentation = model.waveform.presentation();
        let image_preview = model.waveform.image_preview();
        let signal_chrome = model.waveform_chrome.signal_chrome();
        let signal_tools = model.waveform_chrome.signal_tools();

        Self {
            transport_running: model.transport_running,
            map_active: model.map.active,
            active_rating_filters: model.browser.active_rating_filters,
            active_playback_age_filters: model.browser.active_playback_age_filters,
            marked_filter_active: model.browser.marked_filter_active,
            waveform_selection_milli: transport.selection,
            waveform_slices: model.waveform.slices.clone(),
            waveform_selection_export_flash_nonce: feedback_events.primary_success_nonce,
            waveform_selection_export_failure_flash_nonce: feedback_events.primary_failure_nonce,
            waveform_edit_selection_apply_flash_nonce: feedback_events.secondary_success_nonce,
            waveform_edit_selection_milli: edit_preview.selection,
            waveform_edit_fade_in_end_milli: edit_preview.leading_end_milli,
            waveform_edit_fade_in_end_micros: edit_preview.leading_end_micros,
            waveform_edit_fade_in_mute_start_milli: edit_preview.leading_inner_start_milli,
            waveform_edit_fade_in_mute_start_micros: edit_preview.leading_inner_start_micros,
            waveform_edit_fade_in_curve_milli: edit_preview.leading_curve_milli,
            waveform_edit_fade_out_start_milli: edit_preview.trailing_start_milli,
            waveform_edit_fade_out_start_micros: edit_preview.trailing_start_micros,
            waveform_edit_fade_out_mute_end_milli: edit_preview.trailing_inner_end_milli,
            waveform_edit_fade_out_mute_end_micros: edit_preview.trailing_inner_end_micros,
            waveform_edit_fade_out_curve_milli: edit_preview.trailing_curve_milli,
            waveform_loop_enabled: presentation.repeat_enabled,
            waveform_loop_lock_enabled: signal_tools.lock_enabled,
            waveform_cursor_milli: transport.cursor_milli,
            waveform_playhead_milli: transport.playhead_milli,
            waveform_playhead_micros: transport.resolved_playhead_micros(),
            waveform_view_start_milli: viewport.start_milli,
            waveform_view_end_milli: viewport.end_milli,
            waveform_view_start_micros: viewport.start_micros,
            waveform_view_end_micros: viewport.end_micros,
            waveform_view_start_nanos: viewport.start_nanos,
            waveform_view_end_nanos: viewport.end_nanos,
            waveform_tempo_label: presentation.primary_label,
            waveform_zoom_label: presentation.viewport_label,
            waveform_loaded_label: image_preview.loaded_label,
            waveform_loading: image_preview.loading,
            waveform_image_signature: image_preview.image_signature,
            waveform_transport_hint: signal_chrome.status_hint,
            waveform_compare_anchor_available: signal_chrome.reference_anchor_available,
            waveform_compare_anchor_label: signal_chrome.reference_anchor_label,
            waveform_channel_view: signal_chrome.channel_view.into(),
            waveform_normalized_audition_enabled: signal_tools.audition_enabled,
            waveform_bpm_snap_enabled: signal_tools.primary_snap_enabled,
            waveform_relative_bpm_grid_enabled: signal_tools.relative_grid_enabled,
            waveform_transient_snap_enabled: signal_tools.secondary_snap_enabled,
            waveform_transient_markers_enabled: signal_tools.markers_visible,
            waveform_slice_mode_enabled: signal_tools.review_mode_enabled,
            waveform_exact_duplicate_cleanup_available: signal_tools.cleanup_available,
            status_right: model.status.right.clone(),
        }
    }
}

/// Visual playback-age buckets derived from sample playback history.
pub type PlaybackAgeBucket = list::RecencyBucket;

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

/// Browser chrome copy used by the native shell toolbar and tab strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserChromeModel {
    /// Label for the list tab.
    pub samples_tab_label: String,
    /// Label for the map tab.
    pub map_tab_label: String,
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
            map_tab_label: String::from("Similarity map"),
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

impl AppModel {
    pub fn paired_device_panel(&self) -> &AudioEngineModel {
        &self.audio_engine
    }
}

/// Audio field currently expanded into a picker inside the options panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPickerTargetModel {
    /// Output host/backend picker.
    OutputHost,
    /// Output device picker.
    OutputDevice,
    /// Output sample-rate picker.
    OutputSampleRate,
    /// Input host/backend picker.
    InputHost,
    /// Input device picker.
    InputDevice,
    /// Input sample-rate picker.
    InputSampleRate,
}

/// Raw value carried by one audio picker option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioOptionValueModel {
    /// Output host identifier, or `None` for the system default.
    OutputHost(Option<String>),
    /// Output device name, or `None` for the host default.
    OutputDevice(Option<String>),
    /// Output sample rate in Hz, or `None` for the device default.
    OutputSampleRate(Option<u32>),
    /// Input host identifier, or `None` for the system default.
    InputHost(Option<String>),
    /// Input device name, or `None` for the host default.
    InputDevice(Option<String>),
    /// Input sample rate in Hz, or `None` for the device default.
    InputSampleRate(Option<u32>),
}

/// Output/input audio engine state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioEngineModel {
    /// Compact chip health state.
    pub chip_state: AudioEngineChipStateModel,
    /// Compact chip label shown in the top-right chrome.
    pub chip_label: String,
    /// Optional detail or error text shown inside the options overview.
    pub detail_label: Option<String>,
    /// Output host summary row.
    pub output_host: AudioFieldModel,
    /// Output device summary row.
    pub output_device: AudioFieldModel,
    /// Output sample-rate summary row.
    pub output_sample_rate: AudioFieldModel,
    /// Input host summary row.
    pub input_host: AudioFieldModel,
    /// Input device summary row.
    pub input_device: AudioFieldModel,
    /// Input sample-rate summary row.
    pub input_sample_rate: AudioFieldModel,
    /// Currently expanded picker, or `None` for the overview.
    pub active_picker: Option<AudioPickerTargetModel>,
    /// Output host choices.
    pub output_host_options: Vec<AudioOptionItemModel>,
    /// Output device choices.
    pub output_device_options: Vec<AudioOptionItemModel>,
    /// Output sample-rate choices.
    pub output_sample_rate_options: Vec<AudioOptionItemModel>,
    /// Input host choices.
    pub input_host_options: Vec<AudioOptionItemModel>,
    /// Input device choices.
    pub input_device_options: Vec<AudioOptionItemModel>,
    /// Input sample-rate choices.
    pub input_sample_rate_options: Vec<AudioOptionItemModel>,
}

impl AudioEngineModel {
    pub fn status_state(&self) -> AudioEngineChipStateModel {
        self.chip_state
    }

    pub fn status_label(&self) -> &str {
        &self.chip_label
    }

    pub fn detail_label(&self) -> Option<&str> {
        self.detail_label.as_deref()
    }

    pub fn primary_group(&self) -> &AudioFieldModel {
        &self.output_host
    }

    pub fn primary_item(&self) -> &AudioFieldModel {
        &self.output_device
    }

    pub fn primary_number(&self) -> &AudioFieldModel {
        &self.output_sample_rate
    }

    pub fn secondary_group(&self) -> &AudioFieldModel {
        &self.input_host
    }

    pub fn secondary_item(&self) -> &AudioFieldModel {
        &self.input_device
    }

    pub fn secondary_number(&self) -> &AudioFieldModel {
        &self.input_sample_rate
    }

    pub fn active_picker(&self) -> Option<compat::PairedPickerTargetModel> {
        self.active_picker.map(Into::into)
    }

    pub fn options_for(&self, target: compat::PairedPickerTargetModel) -> &[AudioOptionItemModel] {
        match target {
            compat::PairedPickerTargetModel::PrimaryGroup => &self.output_host_options,
            compat::PairedPickerTargetModel::PrimaryItem => &self.output_device_options,
            compat::PairedPickerTargetModel::PrimaryNumber => &self.output_sample_rate_options,
            compat::PairedPickerTargetModel::SecondaryGroup => &self.input_host_options,
            compat::PairedPickerTargetModel::SecondaryItem => &self.input_device_options,
            compat::PairedPickerTargetModel::SecondaryNumber => &self.input_sample_rate_options,
        }
    }
}

/// Options-panel state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OptionsPanelModel {
    /// Whether the panel is currently visible.
    pub visible: bool,
    /// Current default identifier used by auto rename.
    pub default_identifier: String,
    /// Whether input monitoring is enabled.
    pub input_monitoring_enabled: bool,
    /// Whether rating advances browser focus.
    pub advance_after_rating_enabled: bool,
    /// Whether destructive edits skip confirmation.
    pub destructive_yolo_mode_enabled: bool,
    /// Whether waveform scrolling is inverted.
    pub invert_waveform_scroll_enabled: bool,
    /// Short display label for the configured trash folder, when available.
    pub trash_folder_label: Option<String>,
}

impl OptionsPanelModel {
    /// Return this panel's generic preference/settings state.
    pub fn preference_state(&self) -> PreferencePanelStateModel<4> {
        PreferencePanelStateModel::new(
            self.visible,
            self.default_identifier.clone(),
            [
                self.input_monitoring_enabled,
                self.advance_after_rating_enabled,
                self.destructive_yolo_mode_enabled,
                self.invert_waveform_scroll_enabled,
            ],
            self.trash_folder_label.clone(),
        )
    }
}

/// Prompt types that can block interaction in the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmPromptKind {
    /// Pending destructive waveform edit prompt.
    DestructiveEdit,
    /// Pending browser rename prompt.
    BrowserRename,
    /// Pending folder rename prompt.
    FolderRename,
    /// Pending folder creation prompt.
    FolderCreate,
    /// Pending retained folder-delete restore prompt.
    RestoreRetainedFolderDeletes,
    /// Pending retained folder-delete purge prompt.
    PurgeRetainedFolderDeletes,
    /// Pending options-panel default-identifier prompt.
    OptionsDefaultIdentifier,
}

/// Waveform preview metadata consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformPanelModel {
    /// Display label for the loaded sample, when any.
    pub loaded_label: Option<String>,
    /// Whether a newly focused sample is still loading waveform data.
    pub loading: bool,
    /// Whether a replacement waveform image is still rendering in the background.
    pub image_rendering: bool,
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units (`0..=1_000_000`).
    pub playhead_micros: Option<u32>,
    /// Current waveform selection bounds.
    pub selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub slices: Vec<WaveformSlicePreviewModel>,
    /// One-shot token incremented when a waveform-selection export is queued.
    pub selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    pub selection_export_failure_flash_nonce: u64,
    /// One-shot token incremented when preview edit fades are committed.
    pub edit_selection_apply_flash_nonce: u64,
    /// Current waveform edit-selection bounds.
    pub edit_selection_milli: Option<NormalizedRangeModel>,
    /// End position for the edit fade-in region in normalized milli-units.
    pub edit_fade_in_end_milli: Option<u16>,
    /// End position for the edit fade-in region in normalized micro-units.
    pub edit_fade_in_end_micros: Option<u32>,
    /// Start position for the edit fade-in mute region in normalized milli-units.
    pub edit_fade_in_mute_start_milli: Option<u16>,
    /// Start position for the edit fade-in mute region in normalized micro-units.
    pub edit_fade_in_mute_start_micros: Option<u32>,
    /// Fade-in curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_in_curve_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized milli-units.
    pub edit_fade_out_start_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized micro-units.
    pub edit_fade_out_start_micros: Option<u32>,
    /// End position for the edit fade-out mute region in normalized milli-units.
    pub edit_fade_out_mute_end_milli: Option<u16>,
    /// End position for the edit fade-out mute region in normalized micro-units.
    pub edit_fade_out_mute_end_micros: Option<u32>,
    /// Fade-out curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_out_curve_milli: Option<u16>,
    /// Visible view start in normalized milli-units.
    pub view_start_milli: u16,
    /// Visible view end in normalized milli-units.
    pub view_end_milli: u16,
    /// Visible view start in normalized micro-units (`0..=1_000_000`).
    pub view_start_micros: u32,
    /// Visible view end in normalized micro-units (`0..=1_000_000`).
    pub view_end_micros: u32,
    /// Visible view start in normalized nanounits (`0..=1_000_000_000`).
    pub view_start_nanos: u32,
    /// Visible view end in normalized nanounits (`0..=1_000_000_000`).
    pub view_end_nanos: u32,
    /// Quarter-note beat spacing in normalized micro-units when BPM/grid data is available.
    pub beat_step_micros: Option<u32>,
    /// BPM grid origin in normalized micro-units.
    pub bpm_grid_origin_micros: u32,
    /// Whether loop playback is enabled.
    pub loop_enabled: bool,
    /// Optional tempo label rendered in waveform metadata.
    pub tempo_label: Option<String>,
    /// Optional zoom label rendered in waveform metadata.
    pub zoom_label: Option<String>,
    /// Cached signature for waveform image updates.
    pub waveform_image_signature: Option<u64>,
    /// Optional rasterized waveform payload for rendering the waveform preview.
    pub waveform_image: Option<Arc<ImageRgba>>,
}

impl Default for WaveformPanelModel {
    fn default() -> Self {
        Self {
            loaded_label: None,
            loading: false,
            image_rendering: false,
            cursor_milli: None,
            playhead_milli: None,
            playhead_micros: None,
            selection_milli: None,
            slices: Vec::new(),
            selection_export_flash_nonce: 0,
            selection_export_failure_flash_nonce: 0,
            edit_selection_apply_flash_nonce: 0,
            edit_selection_milli: None,
            edit_fade_in_end_milli: None,
            edit_fade_in_end_micros: None,
            edit_fade_in_mute_start_milli: None,
            edit_fade_in_mute_start_micros: None,
            edit_fade_in_curve_milli: None,
            edit_fade_out_start_milli: None,
            edit_fade_out_start_micros: None,
            edit_fade_out_mute_end_milli: None,
            edit_fade_out_mute_end_micros: None,
            edit_fade_out_curve_milli: None,
            view_start_milli: 0,
            view_end_milli: 1000,
            view_start_micros: 0,
            view_end_micros: 1_000_000,
            view_start_nanos: 0,
            view_end_nanos: 1_000_000_000,
            beat_step_micros: None,
            bpm_grid_origin_micros: 0,
            loop_enabled: false,
            tempo_label: None,
            zoom_label: None,
            waveform_image_signature: None,
            waveform_image: None,
        }
    }
}

impl WaveformPanelModel {
    /// Return this panel's generic normalized timeline viewport.
    pub fn viewport(&self) -> compat::WaveformViewportModel {
        compat::WaveformViewportModel::new(
            self.view_start_milli,
            self.view_end_milli,
            self.view_start_micros,
            self.view_end_micros,
            self.view_start_nanos,
            self.view_end_nanos,
        )
    }

    /// Return this panel's generic timeline transport state.
    pub fn transport(&self) -> compat::WaveformTransportModel {
        compat::WaveformTransportModel::new(
            self.cursor_milli,
            self.playhead_milli,
            self.playhead_micros,
            self.selection_milli.map(Into::into),
        )
    }

    /// Return this panel's generic timeline edit preview.
    pub fn edit_preview(&self) -> compat::WaveformEditPreviewModel {
        compat::WaveformEditPreviewModel::new(
            self.edit_selection_milli.map(Into::into),
            self.edit_fade_in_end_milli,
            self.edit_fade_in_end_micros,
            self.edit_fade_in_mute_start_milli,
            self.edit_fade_in_mute_start_micros,
            self.edit_fade_in_curve_milli,
            self.edit_fade_out_start_milli,
            self.edit_fade_out_start_micros,
            self.edit_fade_out_mute_end_milli,
            self.edit_fade_out_mute_end_micros,
            self.edit_fade_out_curve_milli,
        )
    }

    /// Return this panel's generic timeline feedback events.
    pub fn feedback_events(&self) -> WaveformFeedbackEventsModel {
        WaveformFeedbackEventsModel::new(
            self.selection_export_flash_nonce,
            self.selection_export_failure_flash_nonce,
            self.edit_selection_apply_flash_nonce,
        )
    }

    /// Return this panel's generic timeline presentation state.
    pub fn presentation(&self) -> WaveformPresentationModel {
        WaveformPresentationModel::new(
            self.beat_step_micros,
            self.bpm_grid_origin_micros,
            self.loop_enabled,
            self.tempo_label.clone(),
            self.zoom_label.clone(),
        )
    }

    /// Return this panel's generic retained raster preview.
    pub fn image_preview(&self) -> compat::WaveformImagePreviewModel {
        compat::WaveformImagePreviewModel::new(
            self.loaded_label.clone(),
            self.loading,
            self.image_rendering,
            self.waveform_image_signature,
            self.waveform_image.clone(),
        )
    }
}

/// Waveform chrome copy used by metadata lines and control surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformChromeModel {
    /// Extra transport metadata hint shown alongside waveform labels.
    pub transport_hint: String,
    /// Whether compare-anchor replay is currently available.
    pub compare_anchor_available: bool,
    /// Label for the stored compare anchor, when available.
    pub compare_anchor_label: Option<String>,
    /// Whether loop state is locked against sample-driven auto-updates.
    pub loop_lock_enabled: bool,
    /// Current channel-view mode used by waveform rendering.
    pub channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled for waveform edits.
    pub bpm_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub relative_bpm_grid_enabled: bool,
    /// Whether transient snapping is enabled for waveform edits.
    pub transient_snap_enabled: bool,
    /// Whether transient markers are visible on the waveform.
    pub transient_markers_enabled: bool,
    /// Whether slice mode is currently active.
    pub slice_mode_enabled: bool,
    /// Whether the current slice batch is an exact-duplicate cleanup preview.
    pub exact_duplicate_cleanup_available: bool,
}

impl Default for WaveformChromeModel {
    fn default() -> Self {
        Self {
            transport_hint: String::from("transport idle"),
            compare_anchor_available: false,
            compare_anchor_label: None,
            loop_lock_enabled: false,
            channel_view: WaveformChannelViewModel::Mono,
            normalized_audition_enabled: false,
            bpm_snap_enabled: false,
            relative_bpm_grid_enabled: false,
            transient_snap_enabled: false,
            transient_markers_enabled: true,
            slice_mode_enabled: false,
            exact_duplicate_cleanup_available: false,
        }
    }
}

impl WaveformChromeModel {
    /// Return this chrome model's generic signal visualization display state.
    pub fn signal_chrome(&self) -> compat::WaveformChromeStateModel {
        compat::WaveformChromeStateModel::new(
            self.transport_hint.clone(),
            self.compare_anchor_available,
            self.compare_anchor_label.clone(),
            self.channel_view.into(),
        )
    }

    /// Return this chrome model's generic signal visualization tool state.
    pub fn signal_tools(&self) -> compat::WaveformToolStateModel {
        compat::WaveformToolStateModel::new(
            self.loop_lock_enabled,
            self.normalized_audition_enabled,
            self.bpm_snap_enabled,
            self.relative_bpm_grid_enabled,
            self.transient_snap_enabled,
            self.transient_markers_enabled,
            self.slice_mode_enabled,
            self.exact_duplicate_cleanup_available,
        )
    }
}

/// Extract the numeric BPM portion from one projected tempo label.
pub fn parse_waveform_tempo_number_text(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = number.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
}

/// Logical focus buckets projected into the native runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusContextModel {
    /// No UI surface currently owns keyboard focus.
    #[default]
    None,
    /// The waveform viewer handles navigation and shortcuts.
    Waveform,
    /// The sample browser handles row navigation and browser shortcuts.
    SampleBrowser,
    /// The folder tree handles folder navigation and folder shortcuts.
    SourceFolders,
    /// The source list handles source-row navigation and shortcuts.
    SourcesList,
}

/// Sidebar model for source browsing controls.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourcesPanelModel {
    /// Header text for the source panel.
    pub header: String,
    /// Active source-search query.
    pub search_query: String,
    /// Pane that currently drives browser and waveform state.
    pub active_folder_pane: FolderPaneIdModel,
    /// Upper fixed folder pane.
    pub upper_folder_pane: FolderPaneModel,
    /// Lower fixed folder pane.
    pub lower_folder_pane: FolderPaneModel,
    /// Active folder-search query.
    pub tree_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_items: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_items: bool,
    /// Whether folder filtering includes descendant files in a flattened list.
    pub flattened_view: bool,
    /// Whether the folder flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
    /// Selected row index, if any.
    pub selected_row: Option<usize>,
    /// Source row currently hydrating in the background, if any.
    pub loading_row: Option<usize>,
    /// Source row currently running a background file or folder mutation, if any.
    pub mutation_busy_row: Option<usize>,
    /// Focused folder row index, if any.
    pub focused_tree_row: Option<usize>,
    /// Rows to render in the source panel.
    pub rows: RetainedVec<SourceRowModel>,
    /// Folder rows to render in the folder browser section.
    pub tree_rows: RetainedVec<FolderRowModel>,
    /// Folder action availability for native sidebar controls.
    pub tree_actions: FolderActionsModel,
    /// Folder delete-recovery summary for native sidebar status.
    pub recovery: FolderRecoveryModel,
}

impl SourcesPanelModel {
    /// Borrow one pane model by id.
    pub fn folder_pane(&self, pane: FolderPaneIdModel) -> &FolderPaneModel {
        pane.select(&self.upper_folder_pane, &self.lower_folder_pane)
    }

    /// Borrow the pane that currently drives browser and waveform state.
    pub fn active_folder_pane_model(&self) -> &FolderPaneModel {
        self.folder_pane(self.active_folder_pane)
    }

    /// Return this source/sidebar model as a generic split-pane sidebar state.
    pub fn split_pane_sidebar(
        &self,
    ) -> panel::SplitPaneSidebarState<SourceRowModel, FolderRowModel> {
        panel::SplitPaneSidebarState {
            header: self.header.clone(),
            search_query: self.search_query.clone(),
            active_pane: self.active_folder_pane,
            upper_pane: self.upper_folder_pane.clone(),
            lower_pane: self.lower_folder_pane.clone(),
            tree_search_query: self.tree_search_query.clone(),
            show_all_items: self.show_all_items,
            can_toggle_show_all_items: self.can_toggle_show_all_items,
            flattened_view: self.flattened_view,
            can_toggle_flattened_view: self.can_toggle_flattened_view,
            selected_row: self.selected_row,
            loading_row: self.loading_row,
            mutation_busy_row: self.mutation_busy_row,
            focused_tree_row: self.focused_tree_row,
            rows: self.rows.clone(),
            tree_rows: self.tree_rows.clone(),
            tree_actions: self.tree_actions.clone(),
            recovery: self.recovery.clone(),
        }
    }
}

/// Snapshot of Sempal state required by the native shell renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct AppModel {
    /// Main title rendered in the top bar.
    pub title: String,
    /// Backend description shown in top-bar metadata.
    pub backend_label: String,
    /// Sidebar header label.
    pub sources_label: String,
    /// Footer status text.
    pub status_text: String,
    /// Structured footer status segments used by the native shell footer.
    pub status: StatusBarModel,
    /// Output/input audio engine state rendered in the top-right chrome and options panel.
    pub audio_engine: AudioEngineModel,
    /// Browser action availability for native action surfaces.
    pub browser_actions: BrowserActionsModel,
    /// Options-panel overlay projection.
    pub options_panel: OptionsPanelModel,
    /// Progress overlay projection.
    pub progress_overlay: ProgressOverlayModel,
    /// Modal confirm prompt projection.
    pub confirm_prompt: ConfirmPromptModel,
    /// Drag/drop overlay projection.
    pub drag_overlay: DragOverlayModel,
    /// Logical triage/browser columns.
    pub columns: [ColumnModel; 3],
    /// Selected column index (0..=2).
    pub selected_column: usize,
    /// Master output volume normalized to `0.0..=1.0`.
    pub volume: f32,
    /// Whether transport/animation should be considered running.
    pub transport_running: bool,
    /// Source panel model consumed by the native renderer.
    pub sources: SourcesPanelModel,
    /// Browser panel summary consumed by the native renderer.
    pub browser: BrowserPanelModel,
    /// Browser chrome labels consumed by native tabs/toolbar/footer text.
    pub browser_chrome: BrowserChromeModel,
    /// Map panel summary consumed by the native renderer.
    pub map: MapPanelModel,
    /// Waveform panel summary consumed by the native renderer.
    pub waveform: WaveformPanelModel,
    /// Waveform chrome labels consumed by the native waveform header.
    pub waveform_chrome: WaveformChromeModel,
    /// Update surface summary consumed by the native top bar.
    pub update: UpdatePanelModel,
    /// Current keyboard focus bucket used for contextual native key routing.
    pub focus_context: FocusContextModel,
}

impl Default for AppModel {
    fn default() -> Self {
        let mut model = Self::from(compat::AppModel::default());
        model.columns = [
            ColumnModel::new("Trash", 0),
            ColumnModel::new("Samples", 0),
            ColumnModel::new("Keep", 0),
        ];
        model.browser_chrome = BrowserChromeModel::default();
        model
    }
}

fn retained_vec_from_compat<T, U>(value: compat::RetainedVec<T>) -> RetainedVec<U>
where
    T: Clone + Into<U>,
{
    value
        .as_slice()
        .iter()
        .cloned()
        .map(Into::into)
        .collect::<Vec<_>>()
        .into()
}

fn retained_vec_to_compat<T, U>(value: RetainedVec<T>) -> compat::RetainedVec<U>
where
    T: Clone + Into<U>,
{
    value
        .as_slice()
        .iter()
        .cloned()
        .map(Into::into)
        .collect::<Vec<_>>()
        .into()
}

impl From<compat::FocusContextModel> for FocusContextModel {
    fn from(value: compat::FocusContextModel) -> Self {
        match value {
            compat::FocusContextModel::None => Self::None,
            compat::FocusContextModel::Timeline => Self::Waveform,
            compat::FocusContextModel::ContentList => Self::SampleBrowser,
            compat::FocusContextModel::NavigationTree => Self::SourceFolders,
            compat::FocusContextModel::NavigationList => Self::SourcesList,
        }
    }
}

impl From<FocusContextModel> for compat::FocusContextModel {
    fn from(value: FocusContextModel) -> Self {
        match value {
            FocusContextModel::None => Self::None,
            FocusContextModel::Waveform => Self::Timeline,
            FocusContextModel::SampleBrowser => Self::ContentList,
            FocusContextModel::SourceFolders => Self::NavigationTree,
            FocusContextModel::SourcesList => Self::NavigationList,
        }
    }
}

impl From<compat::SourcesPanelModel> for SourcesPanelModel {
    fn from(value: compat::SourcesPanelModel) -> Self {
        Self {
            header: value.header,
            search_query: value.search_query,
            active_folder_pane: value.active_folder_pane.into(),
            upper_folder_pane: value.upper_folder_pane.into(),
            lower_folder_pane: value.lower_folder_pane.into(),
            tree_search_query: value.tree_search_query,
            show_all_items: value.show_all_items,
            can_toggle_show_all_items: value.can_toggle_show_all_items,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            selected_row: value.selected_row,
            loading_row: value.loading_row,
            mutation_busy_row: value.mutation_busy_row,
            focused_tree_row: value.focused_tree_row,
            rows: retained_vec_from_compat(value.rows),
            tree_rows: retained_vec_from_compat(value.tree_rows),
            tree_actions: value.tree_actions.into(),
            recovery: value.recovery.into(),
        }
    }
}

impl From<SourcesPanelModel> for compat::SourcesPanelModel {
    fn from(value: SourcesPanelModel) -> Self {
        Self {
            header: value.header,
            search_query: value.search_query,
            active_folder_pane: value.active_folder_pane.into(),
            upper_folder_pane: value.upper_folder_pane.into(),
            lower_folder_pane: value.lower_folder_pane.into(),
            tree_search_query: value.tree_search_query,
            show_all_items: value.show_all_items,
            can_toggle_show_all_items: value.can_toggle_show_all_items,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            selected_row: value.selected_row,
            loading_row: value.loading_row,
            mutation_busy_row: value.mutation_busy_row,
            focused_tree_row: value.focused_tree_row,
            rows: retained_vec_to_compat(value.rows),
            tree_rows: retained_vec_to_compat(value.tree_rows),
            tree_actions: value.tree_actions.into(),
            recovery: value.recovery.into(),
        }
    }
}

impl From<&SourcesPanelModel> for compat::SourcesPanelModel {
    fn from(value: &SourcesPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserPanelModel> for BrowserPanelModel {
    fn from(value: compat::BrowserPanelModel) -> Self {
        Self {
            visible_count: value.visible_count,
            selected_visible_row: value.selected_visible_row,
            autoscroll: value.autoscroll,
            view_start_row: value.view_start_row,
            selected_path_count: value.selected_item_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_recency_filters,
            marked_filter_active: value.marked_filter_active,
            tag_named_filter_active: value.derived_label_filter_active,
            tag_named_filter_negated: value.derived_label_filter_negated,
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            source_loading: value.data_loading,
            metadata_pending: value.metadata_pending,
            file_op_pending: value.mutation_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_sample_label: value.focused_item_label,
            tag_sidebar: value.pill_editor.into(),
            anchor_visible_row: value.anchor_visible_row,
            rows: retained_vec_from_compat(value.rows),
        }
    }
}

impl From<BrowserPanelModel> for compat::BrowserPanelModel {
    fn from(value: BrowserPanelModel) -> Self {
        Self {
            visible_count: value.visible_count,
            selected_visible_row: value.selected_visible_row,
            autoscroll: value.autoscroll,
            view_start_row: value.view_start_row,
            selected_item_count: value.selected_path_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_recency_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            derived_label_filter_active: value.tag_named_filter_active,
            derived_label_filter_negated: value.tag_named_filter_negated,
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            data_loading: value.source_loading,
            metadata_pending: value.metadata_pending,
            mutation_pending: value.file_op_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_item_label: value.focused_sample_label,
            pill_editor: value.tag_sidebar.into(),
            anchor_visible_row: value.anchor_visible_row,
            rows: retained_vec_to_compat(value.rows),
        }
    }
}

impl From<&BrowserPanelModel> for compat::BrowserPanelModel {
    fn from(value: &BrowserPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserChromeModel> for BrowserChromeModel {
    fn from(value: compat::BrowserChromeModel) -> Self {
        Self {
            samples_tab_label: value.items_tab_label,
            map_tab_label: value.map_tab_label,
            search_prefix_label: value.search_prefix_label,
            search_placeholder: value.search_placeholder,
            activity_ready_label: value.activity_ready_label,
            activity_busy_label: value.activity_busy_label,
            sort_prefix_label: value.sort_prefix_label,
            sort_order_label: value.sort_order_label,
            similarity_toggle_label: value.similarity_toggle_label,
            item_count_label: value.item_count_label,
        }
    }
}

impl From<BrowserChromeModel> for compat::BrowserChromeModel {
    fn from(value: BrowserChromeModel) -> Self {
        Self {
            items_tab_label: value.samples_tab_label,
            map_tab_label: value.map_tab_label,
            search_prefix_label: value.search_prefix_label,
            search_placeholder: value.search_placeholder,
            activity_ready_label: value.activity_ready_label,
            activity_busy_label: value.activity_busy_label,
            sort_prefix_label: value.sort_prefix_label,
            sort_order_label: value.sort_order_label,
            similarity_toggle_label: value.similarity_toggle_label,
            item_count_label: value.item_count_label,
        }
    }
}

impl From<&BrowserChromeModel> for compat::BrowserChromeModel {
    fn from(value: &BrowserChromeModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserActionsModel> for BrowserActionsModel {
    fn from(value: compat::BrowserActionsModel) -> Self {
        Self {
            can_rename: value.can_rename,
            can_delete: value.can_delete,
            can_tag: value.can_edit_pills,
            can_normalize_focused_sample: value.can_process_focused_item,
            can_loop_crossfade_focused_sample: value.can_open_focused_item_flow,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            tag_sidebar_open: value.pill_editor_open,
        }
    }
}

impl From<BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: BrowserActionsModel) -> Self {
        Self {
            can_rename: value.can_rename,
            can_delete: value.can_delete,
            can_edit_pills: value.can_tag,
            can_process_focused_item: value.can_normalize_focused_sample,
            can_open_focused_item_flow: value.can_loop_crossfade_focused_sample,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            pill_editor_open: value.tag_sidebar_open,
        }
    }
}

impl From<&BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: &BrowserActionsModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::PairedPickerTargetModel> for AudioPickerTargetModel {
    fn from(value: compat::PairedPickerTargetModel) -> Self {
        match value {
            compat::PairedPickerTargetModel::PrimaryGroup => Self::OutputHost,
            compat::PairedPickerTargetModel::PrimaryItem => Self::OutputDevice,
            compat::PairedPickerTargetModel::PrimaryNumber => Self::OutputSampleRate,
            compat::PairedPickerTargetModel::SecondaryGroup => Self::InputHost,
            compat::PairedPickerTargetModel::SecondaryItem => Self::InputDevice,
            compat::PairedPickerTargetModel::SecondaryNumber => Self::InputSampleRate,
        }
    }
}

impl From<AudioPickerTargetModel> for compat::PairedPickerTargetModel {
    fn from(value: AudioPickerTargetModel) -> Self {
        match value {
            AudioPickerTargetModel::OutputHost => Self::PrimaryGroup,
            AudioPickerTargetModel::OutputDevice => Self::PrimaryItem,
            AudioPickerTargetModel::OutputSampleRate => Self::PrimaryNumber,
            AudioPickerTargetModel::InputHost => Self::SecondaryGroup,
            AudioPickerTargetModel::InputDevice => Self::SecondaryItem,
            AudioPickerTargetModel::InputSampleRate => Self::SecondaryNumber,
        }
    }
}

impl From<compat::PairedPickerValueModel> for AudioOptionValueModel {
    fn from(value: compat::PairedPickerValueModel) -> Self {
        match value {
            compat::PairedPickerValueModel::PrimaryGroup(value) => Self::OutputHost(value),
            compat::PairedPickerValueModel::PrimaryItem(value) => Self::OutputDevice(value),
            compat::PairedPickerValueModel::PrimaryNumber(value) => Self::OutputSampleRate(value),
            compat::PairedPickerValueModel::SecondaryGroup(value) => Self::InputHost(value),
            compat::PairedPickerValueModel::SecondaryItem(value) => Self::InputDevice(value),
            compat::PairedPickerValueModel::SecondaryNumber(value) => Self::InputSampleRate(value),
        }
    }
}

impl From<AudioOptionValueModel> for compat::PairedPickerValueModel {
    fn from(value: AudioOptionValueModel) -> Self {
        match value {
            AudioOptionValueModel::OutputHost(value) => Self::PrimaryGroup(value),
            AudioOptionValueModel::OutputDevice(value) => Self::PrimaryItem(value),
            AudioOptionValueModel::OutputSampleRate(value) => Self::PrimaryNumber(value),
            AudioOptionValueModel::InputHost(value) => Self::SecondaryGroup(value),
            AudioOptionValueModel::InputDevice(value) => Self::SecondaryItem(value),
            AudioOptionValueModel::InputSampleRate(value) => Self::SecondaryNumber(value),
        }
    }
}

fn audio_option_item_from_compat(value: compat::PairedPickerOptionModel) -> AudioOptionItemModel {
    AudioOptionItemModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

fn audio_option_item_to_compat(value: AudioOptionItemModel) -> compat::PairedPickerOptionModel {
    compat::PairedPickerOptionModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

impl From<compat::PairedDevicePanelModel> for AudioEngineModel {
    fn from(value: compat::PairedDevicePanelModel) -> Self {
        Self {
            chip_state: value.status_state.into(),
            chip_label: value.status_label,
            detail_label: value.detail_label,
            output_host: value.primary_group.into(),
            output_device: value.primary_item.into(),
            output_sample_rate: value.primary_number.into(),
            input_host: value.secondary_group.into(),
            input_device: value.secondary_item.into(),
            input_sample_rate: value.secondary_number.into(),
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .primary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_device_options: value
                .primary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_sample_rate_options: value
                .primary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_host_options: value
                .secondary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_device_options: value
                .secondary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_sample_rate_options: value
                .secondary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
        }
    }
}

impl From<AudioEngineModel> for compat::PairedDevicePanelModel {
    fn from(value: AudioEngineModel) -> Self {
        Self {
            status_state: value.chip_state.into(),
            status_label: value.chip_label,
            detail_label: value.detail_label,
            primary_group: value.output_host.into(),
            primary_item: value.output_device.into(),
            primary_number: value.output_sample_rate.into(),
            secondary_group: value.input_host.into(),
            secondary_item: value.input_device.into(),
            secondary_number: value.input_sample_rate.into(),
            active_picker: value.active_picker.map(Into::into),
            primary_group_options: value
                .output_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_item_options: value
                .output_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_number_options: value
                .output_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_group_options: value
                .input_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_item_options: value
                .input_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_number_options: value
                .input_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
        }
    }
}

impl From<&AudioEngineModel> for compat::PairedDevicePanelModel {
    fn from(value: &AudioEngineModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::OptionsPanelModel> for OptionsPanelModel {
    fn from(value: compat::OptionsPanelModel) -> Self {
        Self {
            visible: value.visible,
            default_identifier: value.default_identifier,
            input_monitoring_enabled: value.input_monitoring_enabled,
            advance_after_rating_enabled: value.advance_after_rating_enabled,
            destructive_yolo_mode_enabled: value.destructive_yolo_mode_enabled,
            invert_waveform_scroll_enabled: value.invert_waveform_scroll_enabled,
            trash_folder_label: value.trash_folder_label,
        }
    }
}

impl From<OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: OptionsPanelModel) -> Self {
        Self {
            visible: value.visible,
            default_identifier: value.default_identifier,
            input_monitoring_enabled: value.input_monitoring_enabled,
            advance_after_rating_enabled: value.advance_after_rating_enabled,
            destructive_yolo_mode_enabled: value.destructive_yolo_mode_enabled,
            invert_waveform_scroll_enabled: value.invert_waveform_scroll_enabled,
            trash_folder_label: value.trash_folder_label,
        }
    }
}

impl From<&OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: &OptionsPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::ConfirmPromptKind> for ConfirmPromptKind {
    fn from(value: compat::ConfirmPromptKind) -> Self {
        match value {
            compat::ConfirmPromptKind::DestructiveOperation => Self::DestructiveEdit,
            compat::ConfirmPromptKind::RenameContent => Self::BrowserRename,
            compat::ConfirmPromptKind::RenameNavigationItem => Self::FolderRename,
            compat::ConfirmPromptKind::CreateNavigationItem => Self::FolderCreate,
            compat::ConfirmPromptKind::RestoreRetainedItems => Self::RestoreRetainedFolderDeletes,
            compat::ConfirmPromptKind::PurgeRetainedItems => Self::PurgeRetainedFolderDeletes,
            compat::ConfirmPromptKind::EditConfiguration => Self::OptionsDefaultIdentifier,
        }
    }
}

impl From<ConfirmPromptKind> for compat::ConfirmPromptKind {
    fn from(value: ConfirmPromptKind) -> Self {
        match value {
            ConfirmPromptKind::DestructiveEdit => Self::DestructiveOperation,
            ConfirmPromptKind::BrowserRename => Self::RenameContent,
            ConfirmPromptKind::FolderRename => Self::RenameNavigationItem,
            ConfirmPromptKind::FolderCreate => Self::CreateNavigationItem,
            ConfirmPromptKind::RestoreRetainedFolderDeletes => Self::RestoreRetainedItems,
            ConfirmPromptKind::PurgeRetainedFolderDeletes => Self::PurgeRetainedItems,
            ConfirmPromptKind::OptionsDefaultIdentifier => Self::EditConfiguration,
        }
    }
}

fn confirm_prompt_from_compat(value: compat::ConfirmPromptModel) -> ConfirmPromptModel {
    ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

fn confirm_prompt_to_compat(value: ConfirmPromptModel) -> compat::ConfirmPromptModel {
    compat::ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

impl From<compat::WaveformPanelModel> for WaveformPanelModel {
    fn from(value: compat::WaveformPanelModel) -> Self {
        Self {
            loaded_label: value.loaded_label,
            loading: value.loading,
            image_rendering: value.image_rendering,
            cursor_milli: value.cursor_milli,
            playhead_milli: value.playhead_milli,
            playhead_micros: value.playhead_micros,
            selection_milli: value.selection_milli.map(Into::into),
            slices: value.slices.into_iter().map(Into::into).collect(),
            selection_export_flash_nonce: value.selection_export_flash_nonce,
            selection_export_failure_flash_nonce: value.selection_export_failure_flash_nonce,
            edit_selection_apply_flash_nonce: value.edit_selection_apply_flash_nonce,
            edit_selection_milli: value.edit_selection_milli.map(Into::into),
            edit_fade_in_end_milli: value.edit_fade_in_end_milli,
            edit_fade_in_end_micros: value.edit_fade_in_end_micros,
            edit_fade_in_mute_start_milli: value.edit_fade_in_mute_start_milli,
            edit_fade_in_mute_start_micros: value.edit_fade_in_mute_start_micros,
            edit_fade_in_curve_milli: value.edit_fade_in_curve_milli,
            edit_fade_out_start_milli: value.edit_fade_out_start_milli,
            edit_fade_out_start_micros: value.edit_fade_out_start_micros,
            edit_fade_out_mute_end_milli: value.edit_fade_out_mute_end_milli,
            edit_fade_out_mute_end_micros: value.edit_fade_out_mute_end_micros,
            edit_fade_out_curve_milli: value.edit_fade_out_curve_milli,
            view_start_milli: value.view_start_milli,
            view_end_milli: value.view_end_milli,
            view_start_micros: value.view_start_micros,
            view_end_micros: value.view_end_micros,
            view_start_nanos: value.view_start_nanos,
            view_end_nanos: value.view_end_nanos,
            beat_step_micros: value.beat_step_micros,
            bpm_grid_origin_micros: value.bpm_grid_origin_micros,
            loop_enabled: value.loop_enabled,
            tempo_label: value.tempo_label,
            zoom_label: value.zoom_label,
            waveform_image_signature: value.waveform_image_signature,
            waveform_image: value.waveform_image,
        }
    }
}

impl From<WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: WaveformPanelModel) -> Self {
        Self {
            loaded_label: value.loaded_label,
            loading: value.loading,
            image_rendering: value.image_rendering,
            cursor_milli: value.cursor_milli,
            playhead_milli: value.playhead_milli,
            playhead_micros: value.playhead_micros,
            selection_milli: value.selection_milli.map(Into::into),
            slices: value.slices.into_iter().map(Into::into).collect(),
            selection_export_flash_nonce: value.selection_export_flash_nonce,
            selection_export_failure_flash_nonce: value.selection_export_failure_flash_nonce,
            edit_selection_apply_flash_nonce: value.edit_selection_apply_flash_nonce,
            edit_selection_milli: value.edit_selection_milli.map(Into::into),
            edit_fade_in_end_milli: value.edit_fade_in_end_milli,
            edit_fade_in_end_micros: value.edit_fade_in_end_micros,
            edit_fade_in_mute_start_milli: value.edit_fade_in_mute_start_milli,
            edit_fade_in_mute_start_micros: value.edit_fade_in_mute_start_micros,
            edit_fade_in_curve_milli: value.edit_fade_in_curve_milli,
            edit_fade_out_start_milli: value.edit_fade_out_start_milli,
            edit_fade_out_start_micros: value.edit_fade_out_start_micros,
            edit_fade_out_mute_end_milli: value.edit_fade_out_mute_end_milli,
            edit_fade_out_mute_end_micros: value.edit_fade_out_mute_end_micros,
            edit_fade_out_curve_milli: value.edit_fade_out_curve_milli,
            view_start_milli: value.view_start_milli,
            view_end_milli: value.view_end_milli,
            view_start_micros: value.view_start_micros,
            view_end_micros: value.view_end_micros,
            view_start_nanos: value.view_start_nanos,
            view_end_nanos: value.view_end_nanos,
            beat_step_micros: value.beat_step_micros,
            bpm_grid_origin_micros: value.bpm_grid_origin_micros,
            loop_enabled: value.loop_enabled,
            tempo_label: value.tempo_label,
            zoom_label: value.zoom_label,
            waveform_image_signature: value.waveform_image_signature,
            waveform_image: value.waveform_image,
        }
    }
}

impl From<&WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: &WaveformPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::WaveformChromeModel> for WaveformChromeModel {
    fn from(value: compat::WaveformChromeModel) -> Self {
        Self {
            transport_hint: value.transport_hint,
            compare_anchor_available: value.compare_anchor_available,
            compare_anchor_label: value.compare_anchor_label,
            loop_lock_enabled: value.loop_lock_enabled,
            channel_view: value.channel_view.into(),
            normalized_audition_enabled: value.normalized_audition_enabled,
            bpm_snap_enabled: value.bpm_snap_enabled,
            relative_bpm_grid_enabled: value.relative_bpm_grid_enabled,
            transient_snap_enabled: value.transient_snap_enabled,
            transient_markers_enabled: value.transient_markers_enabled,
            slice_mode_enabled: value.slice_mode_enabled,
            exact_duplicate_cleanup_available: value.exact_duplicate_cleanup_available,
        }
    }
}

impl From<WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: WaveformChromeModel) -> Self {
        Self {
            transport_hint: value.transport_hint,
            compare_anchor_available: value.compare_anchor_available,
            compare_anchor_label: value.compare_anchor_label,
            loop_lock_enabled: value.loop_lock_enabled,
            channel_view: value.channel_view.into(),
            normalized_audition_enabled: value.normalized_audition_enabled,
            bpm_snap_enabled: value.bpm_snap_enabled,
            relative_bpm_grid_enabled: value.relative_bpm_grid_enabled,
            transient_snap_enabled: value.transient_snap_enabled,
            transient_markers_enabled: value.transient_markers_enabled,
            slice_mode_enabled: value.slice_mode_enabled,
            exact_duplicate_cleanup_available: value.exact_duplicate_cleanup_available,
        }
    }
}

impl From<&WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: &WaveformChromeModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::AppModel> for AppModel {
    fn from(value: compat::AppModel) -> Self {
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status.into(),
            audio_engine: value.paired_device.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: confirm_prompt_from_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay.into(),
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources.into(),
            browser: value.browser.into(),
            browser_chrome: value.browser_chrome.into(),
            map: value.map.into(),
            waveform: value.waveform.into(),
            waveform_chrome: value.waveform_chrome.into(),
            update: value.update.into(),
            focus_context: value.focus_context.into(),
        }
    }
}

impl From<AppModel> for compat::AppModel {
    fn from(value: AppModel) -> Self {
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status.into(),
            paired_device: value.audio_engine.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: confirm_prompt_to_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay.into(),
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources.into(),
            browser: value.browser.into(),
            browser_chrome: value.browser_chrome.into(),
            map: value.map.into(),
            waveform: value.waveform.into(),
            waveform_chrome: value.waveform_chrome.into(),
            update: value.update.into(),
            focus_context: value.focus_context.into(),
        }
    }
}

impl From<&AppModel> for compat::AppModel {
    fn from(value: &AppModel) -> Self {
        value.clone().into()
    }
}

fn automation_node_id_from_compat(value: compat::AutomationNodeId) -> AutomationNodeId {
    automation::AutomationNodeId(automation_node_id_string_from_compat(value.0))
}

fn automation_node_id_to_compat(value: AutomationNodeId) -> compat::AutomationNodeId {
    compat::AutomationNodeId(automation_node_id_string_to_compat(value.0))
}

fn automation_node_id_string_from_compat(node_id: String) -> String {
    match node_id.as_str() {
        "browser.pill_editor" => String::from("browser.tag_sidebar"),
        "browser.pill_editor.input" => String::from("browser.tag_sidebar.input"),
        "browser.pill_editor.exclusive.0" => String::from("browser.tag_sidebar.playback.loop"),
        "browser.pill_editor.exclusive.1" => String::from("browser.tag_sidebar.playback.one_shot"),
        _ => {
            if let Some(suffix) = node_id.strip_prefix("browser.pill_editor.option.") {
                format!("browser.tag_sidebar.normal_tag.{suffix}")
            } else if let Some(suffix) = node_id.strip_prefix("browser.pill_editor.create.") {
                format!("browser.tag_sidebar.create_tag.{suffix}")
            } else {
                node_id
            }
        }
    }
}

fn automation_node_id_string_to_compat(node_id: String) -> String {
    match node_id.as_str() {
        "browser.tag_sidebar" => String::from("browser.pill_editor"),
        "browser.tag_sidebar.input" => String::from("browser.pill_editor.input"),
        "browser.tag_sidebar.playback.loop" => String::from("browser.pill_editor.exclusive.0"),
        "browser.tag_sidebar.playback.one_shot" => String::from("browser.pill_editor.exclusive.1"),
        _ => {
            if let Some(suffix) = node_id.strip_prefix("browser.tag_sidebar.normal_tag.") {
                format!("browser.pill_editor.option.{suffix}")
            } else if let Some(suffix) = node_id.strip_prefix("browser.tag_sidebar.create_tag.") {
                format!("browser.pill_editor.create.{suffix}")
            } else {
                node_id
            }
        }
    }
}

impl From<compat::AutomationRole> for AutomationRole {
    fn from(value: compat::AutomationRole) -> Self {
        match value {
            compat::AutomationRole::Root => Self::Root,
            compat::AutomationRole::Group => Self::Group,
            compat::AutomationRole::Panel => Self::Panel,
            compat::AutomationRole::Toolbar => Self::Toolbar,
            compat::AutomationRole::TabList => Self::TabList,
            compat::AutomationRole::Tab => Self::Tab,
            compat::AutomationRole::Button => Self::Button,
            compat::AutomationRole::SearchField => Self::SearchField,
            compat::AutomationRole::Slider => Self::Slider,
            compat::AutomationRole::Row => Self::Row,
            compat::AutomationRole::Table => Self::Table,
            compat::AutomationRole::TimelineRegion => Self::WaveformRegion,
            compat::AutomationRole::SpatialCanvas => Self::MapCanvas,
            compat::AutomationRole::SpatialPoint => Self::MapPoint,
            compat::AutomationRole::Readout => Self::Readout,
            compat::AutomationRole::Dialog => Self::Dialog,
        }
    }
}

impl From<AutomationRole> for compat::AutomationRole {
    fn from(value: AutomationRole) -> Self {
        match value {
            AutomationRole::Root => Self::Root,
            AutomationRole::Group => Self::Group,
            AutomationRole::Panel => Self::Panel,
            AutomationRole::Toolbar => Self::Toolbar,
            AutomationRole::TabList => Self::TabList,
            AutomationRole::Tab => Self::Tab,
            AutomationRole::Button => Self::Button,
            AutomationRole::SearchField => Self::SearchField,
            AutomationRole::Slider => Self::Slider,
            AutomationRole::Row => Self::Row,
            AutomationRole::Table => Self::Table,
            AutomationRole::WaveformRegion => Self::TimelineRegion,
            AutomationRole::MapCanvas => Self::SpatialCanvas,
            AutomationRole::MapPoint => Self::SpatialPoint,
            AutomationRole::Readout => Self::Readout,
            AutomationRole::Dialog => Self::Dialog,
        }
    }
}

fn automation_bounds_from_compat(value: compat::AutomationBounds) -> AutomationBounds {
    AutomationBounds {
        x: value.x,
        y: value.y,
        width: value.width,
        height: value.height,
    }
}

fn automation_bounds_to_compat(value: AutomationBounds) -> compat::AutomationBounds {
    compat::AutomationBounds {
        x: value.x,
        y: value.y,
        width: value.width,
        height: value.height,
    }
}

impl From<compat::AutomationNodeSnapshot> for AutomationNodeSnapshot {
    fn from(value: compat::AutomationNodeSnapshot) -> Self {
        Self {
            id: automation_node_id_from_compat(value.id),
            role: value.role.into(),
            label: value.label,
            bounds: automation_bounds_from_compat(value.bounds),
            value: value.value,
            enabled: value.enabled,
            selected: value.selected,
            available_actions: value
                .available_actions
                .into_iter()
                .map(automation_action_id_from_compat)
                .collect(),
            metadata: automation_metadata_from_compat(value.metadata),
            children: value.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<AutomationNodeSnapshot> for compat::AutomationNodeSnapshot {
    fn from(value: AutomationNodeSnapshot) -> Self {
        Self {
            id: automation_node_id_to_compat(value.id),
            role: value.role.into(),
            label: value.label,
            bounds: automation_bounds_to_compat(value.bounds),
            value: value.value,
            enabled: value.enabled,
            selected: value.selected,
            available_actions: value
                .available_actions
                .into_iter()
                .map(automation_action_id_to_compat)
                .collect(),
            metadata: automation_metadata_to_compat(value.metadata),
            children: value.children.into_iter().map(Into::into).collect(),
        }
    }
}

fn automation_action_id_from_compat(action_id: String) -> String {
    match action_id.as_str() {
        "open_primary_group_picker" => String::from("open_audio_output_host_picker"),
        "open_primary_item_picker" => String::from("open_audio_output_device_picker"),
        "open_primary_number_picker" => String::from("open_audio_output_sample_rate_picker"),
        "open_secondary_group_picker" => String::from("open_audio_input_host_picker"),
        "open_secondary_item_picker" => String::from("open_audio_input_device_picker"),
        "open_secondary_number_picker" => String::from("open_audio_input_sample_rate_picker"),
        "set_primary_group" => String::from("set_audio_output_host"),
        "set_primary_item" => String::from("set_audio_output_device"),
        "set_primary_number" => String::from("set_audio_output_sample_rate"),
        "set_secondary_group" => String::from("set_audio_input_host"),
        "set_secondary_item" => String::from("set_audio_input_device"),
        "set_secondary_number" => String::from("set_audio_input_sample_rate"),
        "focus_spatial_content_item" => String::from("focus_map_sample"),
        "focus_browser_pill_editor_input" => String::from("focus_browser_tag_sidebar_input"),
        "set_browser_pill_editor_input" => String::from("set_browser_tag_sidebar_input"),
        "commit_browser_pill_editor_input" => String::from("commit_browser_tag_sidebar_input"),
        "toggle_browser_pill_editor" => String::from("toggle_browser_tag_sidebar"),
        "toggle_browser_pill_editor_primary_action" => {
            String::from("toggle_browser_tag_sidebar_auto_rename")
        }
        "toggle_browser_pill_option" => String::from("toggle_browser_sidebar_normal_tag"),
        "toggle_browser_derived_label_filter" => String::from("toggle_browser_tag_named_filter"),
        _ => action_id,
    }
}

fn automation_action_id_to_compat(action_id: String) -> String {
    match action_id.as_str() {
        "open_audio_output_host_picker" => String::from("open_primary_group_picker"),
        "open_audio_output_device_picker" => String::from("open_primary_item_picker"),
        "open_audio_output_sample_rate_picker" => String::from("open_primary_number_picker"),
        "open_audio_input_host_picker" => String::from("open_secondary_group_picker"),
        "open_audio_input_device_picker" => String::from("open_secondary_item_picker"),
        "open_audio_input_sample_rate_picker" => String::from("open_secondary_number_picker"),
        "set_audio_output_host" => String::from("set_primary_group"),
        "set_audio_output_device" => String::from("set_primary_item"),
        "set_audio_output_sample_rate" => String::from("set_primary_number"),
        "set_audio_input_host" => String::from("set_secondary_group"),
        "set_audio_input_device" => String::from("set_secondary_item"),
        "set_audio_input_sample_rate" => String::from("set_secondary_number"),
        "focus_map_sample" => String::from("focus_spatial_content_item"),
        "focus_browser_tag_sidebar_input" => String::from("focus_browser_pill_editor_input"),
        "set_browser_tag_sidebar_input" => String::from("set_browser_pill_editor_input"),
        "commit_browser_tag_sidebar_input" => String::from("commit_browser_pill_editor_input"),
        "toggle_browser_tag_sidebar" => String::from("toggle_browser_pill_editor"),
        "toggle_browser_tag_sidebar_auto_rename" => {
            String::from("toggle_browser_pill_editor_primary_action")
        }
        "toggle_browser_sidebar_normal_tag" => String::from("toggle_browser_pill_option"),
        "toggle_browser_tag_named_filter" => String::from("toggle_browser_derived_label_filter"),
        _ => action_id,
    }
}

fn automation_metadata_from_compat(
    mut metadata: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if let Some(value) = metadata.remove("focused_item_label") {
        metadata.insert(String::from("focused_sample_label"), value);
    }
    if let Some(value) = metadata.remove("option_pill_labels") {
        metadata.insert(String::from("normal_tag_labels"), value);
    }
    if let Some(value) = metadata.remove("pill_state") {
        metadata.insert(String::from("tag_state"), value);
    }
    if let Some(value) = metadata.remove("pill_id") {
        metadata.insert(String::from("tag_id"), value);
    }
    metadata
}

fn automation_metadata_to_compat(
    mut metadata: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if let Some(value) = metadata.remove("focused_sample_label") {
        metadata.insert(String::from("focused_item_label"), value);
    }
    if let Some(value) = metadata.remove("normal_tag_labels") {
        metadata.insert(String::from("option_pill_labels"), value);
    }
    if let Some(value) = metadata.remove("tag_state") {
        metadata.insert(String::from("pill_state"), value);
    }
    if let Some(value) = metadata.remove("tag_id") {
        metadata.insert(String::from("pill_id"), value);
    }
    metadata
}

impl From<compat::GuiAutomationSnapshot> for GuiAutomationSnapshot {
    fn from(value: compat::GuiAutomationSnapshot) -> Self {
        Self {
            schema_version: value.schema_version,
            viewport_width: value.viewport_width,
            viewport_height: value.viewport_height,
            root: value.root.into(),
        }
    }
}

impl From<GuiAutomationSnapshot> for compat::GuiAutomationSnapshot {
    fn from(value: GuiAutomationSnapshot) -> Self {
        Self {
            schema_version: value.schema_version,
            viewport_width: value.viewport_width,
            viewport_height: value.viewport_height,
            root: value.root.into(),
        }
    }
}

impl From<compat::DirtySegments> for DirtySegments {
    fn from(value: compat::DirtySegments) -> Self {
        Self::from_bits(value.bits())
    }
}

impl From<DirtySegments> for compat::DirtySegments {
    fn from(value: DirtySegments) -> Self {
        Self::from_bits(value.bits())
    }
}

impl From<compat::SegmentRevisions> for SegmentRevisions {
    fn from(value: compat::SegmentRevisions) -> Self {
        Self {
            status_bar: value.status_bar,
            browser_frame: value.browser_frame,
            browser_rows_window: value.browser_rows_window,
            map_panel: value.map_panel,
            waveform_overlay: value.waveform_overlay,
            global_static: value.global_static,
        }
    }
}

impl From<SegmentRevisions> for compat::SegmentRevisions {
    fn from(value: SegmentRevisions) -> Self {
        Self {
            status_bar: value.status_bar,
            browser_frame: value.browser_frame,
            browser_rows_window: value.browser_rows_window,
            map_panel: value.map_panel,
            waveform_overlay: value.waveform_overlay,
            global_static: value.global_static,
        }
    }
}

impl From<compat::NativeMotionModel> for NativeMotionModel {
    fn from(value: compat::NativeMotionModel) -> Self {
        Self {
            transport_running: value.transport_running,
            map_active: value.map_active,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            waveform_selection_milli: value.waveform_selection_milli.map(Into::into),
            waveform_slices: value.waveform_slices.into_iter().map(Into::into).collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli.map(Into::into),
            waveform_edit_fade_in_end_milli: value.waveform_edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: value.waveform_edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: value.waveform_edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: value.waveform_edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: value.waveform_edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: value.waveform_edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: value.waveform_edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: value.waveform_edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: value.waveform_edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: value.waveform_edit_fade_out_curve_milli,
            waveform_loop_enabled: value.waveform_loop_enabled,
            waveform_loop_lock_enabled: value.waveform_loop_lock_enabled,
            waveform_cursor_milli: value.waveform_cursor_milli,
            waveform_playhead_milli: value.waveform_playhead_milli,
            waveform_playhead_micros: value.waveform_playhead_micros,
            waveform_view_start_milli: value.waveform_view_start_milli,
            waveform_view_end_milli: value.waveform_view_end_milli,
            waveform_view_start_micros: value.waveform_view_start_micros,
            waveform_view_end_micros: value.waveform_view_end_micros,
            waveform_view_start_nanos: value.waveform_view_start_nanos,
            waveform_view_end_nanos: value.waveform_view_end_nanos,
            waveform_tempo_label: value.waveform_tempo_label,
            waveform_zoom_label: value.waveform_zoom_label,
            waveform_loaded_label: value.waveform_loaded_label,
            waveform_loading: value.waveform_loading,
            waveform_image_signature: value.waveform_image_signature,
            waveform_transport_hint: value.waveform_transport_hint,
            waveform_compare_anchor_available: value.waveform_compare_anchor_available,
            waveform_compare_anchor_label: value.waveform_compare_anchor_label,
            waveform_channel_view: value.waveform_channel_view.into(),
            waveform_normalized_audition_enabled: value.waveform_normalized_audition_enabled,
            waveform_bpm_snap_enabled: value.waveform_bpm_snap_enabled,
            waveform_relative_bpm_grid_enabled: value.waveform_relative_bpm_grid_enabled,
            waveform_transient_snap_enabled: value.waveform_transient_snap_enabled,
            waveform_transient_markers_enabled: value.waveform_transient_markers_enabled,
            waveform_slice_mode_enabled: value.waveform_slice_mode_enabled,
            waveform_exact_duplicate_cleanup_available: value
                .waveform_exact_duplicate_cleanup_available,
            status_right: value.status_right,
        }
    }
}

impl From<NativeMotionModel> for compat::NativeMotionModel {
    fn from(value: NativeMotionModel) -> Self {
        Self {
            transport_running: value.transport_running,
            map_active: value.map_active,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            waveform_selection_milli: value.waveform_selection_milli.map(Into::into),
            waveform_slices: value.waveform_slices.into_iter().map(Into::into).collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli.map(Into::into),
            waveform_edit_fade_in_end_milli: value.waveform_edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: value.waveform_edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: value.waveform_edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: value.waveform_edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: value.waveform_edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: value.waveform_edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: value.waveform_edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: value.waveform_edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: value.waveform_edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: value.waveform_edit_fade_out_curve_milli,
            waveform_loop_enabled: value.waveform_loop_enabled,
            waveform_loop_lock_enabled: value.waveform_loop_lock_enabled,
            waveform_cursor_milli: value.waveform_cursor_milli,
            waveform_playhead_milli: value.waveform_playhead_milli,
            waveform_playhead_micros: value.waveform_playhead_micros,
            waveform_view_start_milli: value.waveform_view_start_milli,
            waveform_view_end_milli: value.waveform_view_end_milli,
            waveform_view_start_micros: value.waveform_view_start_micros,
            waveform_view_end_micros: value.waveform_view_end_micros,
            waveform_view_start_nanos: value.waveform_view_start_nanos,
            waveform_view_end_nanos: value.waveform_view_end_nanos,
            waveform_tempo_label: value.waveform_tempo_label,
            waveform_zoom_label: value.waveform_zoom_label,
            waveform_loaded_label: value.waveform_loaded_label,
            waveform_loading: value.waveform_loading,
            waveform_image_signature: value.waveform_image_signature,
            waveform_transport_hint: value.waveform_transport_hint,
            waveform_compare_anchor_available: value.waveform_compare_anchor_available,
            waveform_compare_anchor_label: value.waveform_compare_anchor_label,
            waveform_channel_view: value.waveform_channel_view.into(),
            waveform_normalized_audition_enabled: value.waveform_normalized_audition_enabled,
            waveform_bpm_snap_enabled: value.waveform_bpm_snap_enabled,
            waveform_relative_bpm_grid_enabled: value.waveform_relative_bpm_grid_enabled,
            waveform_transient_snap_enabled: value.waveform_transient_snap_enabled,
            waveform_transient_markers_enabled: value.waveform_transient_markers_enabled,
            waveform_slice_mode_enabled: value.waveform_slice_mode_enabled,
            waveform_exact_duplicate_cleanup_available: value
                .waveform_exact_duplicate_cleanup_available,
            status_right: value.status_right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WaveformPanelModel, parse_waveform_tempo_number_text};

    #[test]
    fn waveform_panel_default_bpm_grid_origin_is_zero() {
        assert_eq!(WaveformPanelModel::default().bpm_grid_origin_micros, 0);
    }

    #[test]
    fn options_panel_projects_generic_preference_state() {
        let model = super::OptionsPanelModel {
            visible: true,
            default_identifier: String::from("portal"),
            input_monitoring_enabled: true,
            advance_after_rating_enabled: false,
            destructive_yolo_mode_enabled: true,
            invert_waveform_scroll_enabled: false,
            trash_folder_label: Some(String::from("Trash")),
        };
        let preferences = model.preference_state();

        assert!(preferences.visible);
        assert_eq!(preferences.primary_text_value, "portal");
        assert_eq!(preferences.toggles, [true, false, true, false]);
        assert_eq!(preferences.auxiliary_label.as_deref(), Some("Trash"));
    }

    #[test]
    fn waveform_panel_projects_generic_feedback_events() {
        let model = WaveformPanelModel {
            selection_export_flash_nonce: 11,
            selection_export_failure_flash_nonce: 12,
            edit_selection_apply_flash_nonce: 13,
            ..WaveformPanelModel::default()
        };
        let events = model.feedback_events();

        assert_eq!(events.primary_success_nonce, 11);
        assert_eq!(events.primary_failure_nonce, 12);
        assert_eq!(events.secondary_success_nonce, 13);
    }

    #[test]
    fn waveform_panel_projects_generic_presentation_state() {
        let model = WaveformPanelModel {
            beat_step_micros: Some(100_000),
            bpm_grid_origin_micros: 50_000,
            loop_enabled: true,
            tempo_label: Some(String::from("150 BPM")),
            zoom_label: Some(String::from("8x")),
            ..WaveformPanelModel::default()
        };
        let presentation = model.presentation();

        assert_eq!(presentation.guide_step_micros, Some(100_000));
        assert_eq!(presentation.guide_origin_micros, 50_000);
        assert!(presentation.repeat_enabled);
        assert_eq!(presentation.primary_label.as_deref(), Some("150 BPM"));
        assert_eq!(presentation.viewport_label.as_deref(), Some("8x"));
    }

    #[test]
    fn parse_waveform_tempo_number_text_accepts_integer_and_fractional_labels() {
        assert_eq!(
            parse_waveform_tempo_number_text("128 BPM"),
            Some(String::from("128"))
        );
        assert_eq!(
            parse_waveform_tempo_number_text("128.5 BPM"),
            Some(String::from("128.5"))
        );
    }

    #[test]
    fn parse_waveform_tempo_number_text_rejects_empty_and_invalid_labels() {
        assert_eq!(parse_waveform_tempo_number_text(""), None);
        assert_eq!(parse_waveform_tempo_number_text("0 BPM"), None);
        assert_eq!(parse_waveform_tempo_number_text("-1 BPM"), None);
        assert_eq!(parse_waveform_tempo_number_text("fast BPM"), None);
    }
}
