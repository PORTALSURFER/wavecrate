//! Sempal-owned native shell projection DTOs.
//!
//! These models describe Sempal application state as projected for the current
//! native shell. Radiant still consumes a compatibility copy at the runtime
//! boundary, so this module also provides field-for-field adapters that preserve
//! the legacy shell snapshot contract without making Radiant the owner of the
//! Sempal projection types.

use radiant::compat::sempal_shell as compat;
use radiant::gui::types::ImageRgba;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc};

/// Shared storage used by retained app-model snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetainedVec<T>(Arc<Vec<T>>);

impl<T> RetainedVec<T> {
    /// Build an empty retained vector.
    pub fn new() -> Self {
        Self(Arc::new(Vec::new()))
    }

    /// Append one element, cloning the backing vector only when aliased.
    pub fn push(&mut self, value: T)
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).push(value);
    }

    /// Clear all elements, preserving retained storage when possible.
    pub fn clear(&mut self)
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).clear();
    }

    /// Truncate the vector to `len`.
    pub fn truncate(&mut self, len: usize)
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).truncate(len);
    }

    /// Return the number of retained elements.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return whether the retained vector is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Borrow the retained contents as a slice.
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Borrow one retained element by index.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index)
    }

    /// Borrow one retained element mutably, cloning the backing vector only when aliased.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T>
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0).get_mut(index)
    }

    /// Borrow the backing vector mutably for batched updates.
    pub fn make_mut(&mut self) -> &mut Vec<T>
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0)
    }
}

impl<T> Default for RetainedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for RetainedVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> From<Vec<T>> for RetainedVec<T> {
    fn from(value: Vec<T>) -> Self {
        Self(Arc::new(value))
    }
}

/// Browser playback-age filter chips shown in the native toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaybackAgeFilterChip {
    /// Samples that have never been played.
    NeverPlayed,
    /// Samples whose last playback was at least 30 days ago.
    OlderThanMonth,
    /// Samples whose last playback was at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

/// Visual playback-age buckets derived from sample playback history.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaybackAgeBucket {
    /// Samples played within the last 7 days, including future-skewed timestamps.
    #[default]
    Fresh,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
}

/// Summary of one browser/list row consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRowModel {
    /// Visible row index in the filtered browser list.
    pub visible_row: usize,
    /// Display label for the row.
    pub label: Arc<str>,
    /// Triage column index (`0..=2`) that currently owns the row.
    pub column: usize,
    /// Signed keep/trash rating level shown alongside the row label (`-3..=3`).
    pub rating_level: i8,
    /// Visual playback-age bucket used to render the browser row age marker.
    pub playback_age_bucket: PlaybackAgeBucket,
    /// Optional inline metadata label rendered at the right edge of the sample lane.
    pub bucket_label: Option<Arc<str>>,
    /// Optional normalized similarity fill amount encoded in the inclusive `0..=255` range.
    pub similarity_display_strength: Option<u8>,
    /// Whether this row is currently selected in multi-selection state.
    pub selected: bool,
    /// Whether this row currently has focus/caret.
    pub focused: bool,
    /// Whether the backing sample file is missing on disk.
    pub missing: bool,
    /// Whether the backing sample is marked as a confirmed keep lock.
    pub locked: bool,
    /// Whether the backing sample is session-marked for later review.
    pub marked: bool,
}

impl BrowserRowModel {
    /// Build a row model, clamping the column into `0..=2`.
    pub fn new(
        visible_row: usize,
        label: impl Into<String>,
        column: usize,
        selected: bool,
        focused: bool,
    ) -> Self {
        Self {
            visible_row,
            label: Arc::<str>::from(label.into()),
            column: column.min(2),
            rating_level: 0,
            playback_age_bucket: PlaybackAgeBucket::Fresh,
            bucket_label: None,
            similarity_display_strength: None,
            selected,
            focused,
            missing: false,
            locked: false,
            marked: false,
        }
    }

    /// Attach a signed keep/trash rating level for inline row indicators.
    pub fn with_rating_level(mut self, rating_level: i8) -> Self {
        self.rating_level = rating_level.clamp(-3, 3);
        self
    }

    /// Attach the playback-age bucket used for row aging treatment.
    pub fn with_playback_age_bucket(mut self, playback_age_bucket: PlaybackAgeBucket) -> Self {
        self.playback_age_bucket = playback_age_bucket;
        self
    }

    /// Attach an explicit inline metadata label for this row.
    pub fn with_bucket_label(mut self, label: impl Into<String>) -> Self {
        self.bucket_label = Some(Arc::<str>::from(label.into()));
        self
    }

    /// Attach a normalized similarity display strength for the compact row bar.
    pub fn with_similarity_display_strength(mut self, display_strength: f32) -> Self {
        self.similarity_display_strength =
            Some(Self::encode_similarity_display_strength(display_strength));
        self
    }

    /// Encode one normalized similarity display strength into the stored byte range.
    pub fn encode_similarity_display_strength(display_strength: f32) -> u8 {
        (display_strength.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Decode the stored similarity display strength into a normalized fill amount.
    pub fn similarity_display_strength_ratio(&self) -> Option<f32> {
        self.similarity_display_strength
            .map(|strength| f32::from(strength) / 255.0)
    }

    /// Mark whether the backing sample file is missing on disk.
    pub fn with_missing(mut self, missing: bool) -> Self {
        self.missing = missing;
        self
    }

    /// Mark whether the backing sample should render with the keep-lock highlight.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Mark whether the backing sample should render with the session mark treatment.
    pub fn with_marked(mut self, marked: bool) -> Self {
        self.marked = marked;
        self
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

/// Tri-state pill state used by the browser metadata editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BrowserTagState {
    /// No selected rows currently carry the value.
    #[default]
    Off,
    /// Every selected row currently carries the value.
    On,
    /// Selected rows disagree about the value.
    Mixed,
}

/// One clickable tag pill projected into the browser metadata sidebar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserTagPillModel {
    /// Stable identifier for hit testing and automation.
    pub id: String,
    /// User-facing pill label.
    pub label: String,
    /// Tri-state selection value for the current browser target set.
    pub state: BrowserTagState,
}

/// Browser-local metadata sidebar shown beside the sample list.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserTagSidebarModel {
    /// Whether the sidebar should render in the current browser view.
    pub open: bool,
    /// Count of selected rows represented by the sidebar target set.
    pub selected_count: usize,
    /// Header line describing the current selection/focus context.
    pub header_label: String,
    /// Whether sidebar metadata edits should trigger auto-rename.
    pub auto_rename_enabled: bool,
    /// Current custom-tag input value.
    pub input_value: String,
    /// Placeholder shown for the custom-tag input when empty.
    pub input_placeholder: String,
    /// Exclusive playback-type pills.
    pub playback_type_pills: [BrowserTagPillModel; 2],
    /// Exclusive sound-type pills.
    pub sound_type_pills: Vec<BrowserTagPillModel>,
    /// Active custom-tag pill when present in the selection.
    pub custom_tag_pill: Option<BrowserTagPillModel>,
}

/// Render mode label for the map panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MapRenderModeModel {
    /// Rendered as a density heatmap.
    Heatmap,
    /// Rendered as individual points.
    #[default]
    Points,
}

/// Render data for one map point shown in the native map canvas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPointModel {
    /// Stable sample id used to route click actions back to the host.
    pub sample_id: Arc<str>,
    /// X position normalized to milli-units (`0..=1000`) across map bounds.
    pub x_milli: u16,
    /// Y position normalized to milli-units (`0..=1000`) across map bounds.
    pub y_milli: u16,
    /// Optional cluster id for color grouping.
    pub cluster_id: Option<i32>,
}

/// Summary of map state consumed by the native shell map tab.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MapPanelModel {
    /// Whether the map tab is currently active in the browser panel.
    pub active: bool,
    /// Human-readable map summary line.
    pub summary: String,
    /// Legend/status label for map render mode and point density.
    pub legend_label: String,
    /// Selection/focus label for the currently highlighted map sample.
    pub selection_label: String,
    /// Hover label for the currently hovered map sample, when any.
    pub hover_label: String,
    /// Cluster summary label for projected map points.
    pub cluster_label: String,
    /// Viewport label describing zoom/pan state.
    pub viewport_label: String,
    /// Optional error text shown when map data cannot be loaded.
    pub error: Option<String>,
    /// Current map render mode.
    pub render_mode: MapRenderModeModel,
    /// Sample id currently selected in map state, when any.
    pub selected_sample_id: Option<String>,
    /// Sample id currently focused from the browser list, when any.
    pub focused_sample_id: Option<String>,
    /// Points available for rendering in normalized map space.
    pub points: Arc<[MapPointModel]>,
}

/// Structured footer status content for left/center/right status segments.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StatusBarModel {
    /// Left-aligned status segment.
    pub left: String,
    /// Center-aligned status segment.
    pub center: String,
    /// Right-aligned status segment.
    pub right: String,
}

/// Health state of the compact audio-engine status chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioEngineChipStateModel {
    /// Output engine is active and matches the requested configuration.
    #[default]
    Healthy,
    /// Output engine is unavailable or degraded.
    Error,
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

/// One selectable item shown inside an audio picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOptionItemModel {
    /// Human-readable option label.
    pub label: String,
    /// Whether the option is currently selected.
    pub selected: bool,
    /// Raw value applied when the option is chosen.
    pub value: AudioOptionValueModel,
}

/// Overview row shown for one audio field inside the options panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioFieldModel {
    /// Static row label.
    pub label: String,
    /// Current value summary.
    pub value_label: String,
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

/// Update-check status projected into the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UpdateStatusModel {
    /// No update activity in progress.
    #[default]
    Idle,
    /// Update check is running.
    Checking,
    /// A newer update is available.
    Available,
    /// Update check failed.
    Error,
}

/// Update panel state used by native top-bar actions.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UpdatePanelModel {
    /// Current update-check status.
    pub status: UpdateStatusModel,
    /// Status label rendered in native top-bar chrome.
    pub status_label: String,
    /// Action hint label rendered near update controls.
    pub action_hint_label: String,
    /// Supplemental release-notes label rendered under update hints.
    pub release_notes_label: String,
    /// Available release tag, when present.
    pub available_tag: Option<String>,
    /// Available release URL, when present.
    pub available_url: Option<String>,
    /// Last error message from update checks, if any.
    pub last_error: Option<String>,
}

/// Progress overlay state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProgressOverlayModel {
    /// Whether the overlay is currently visible.
    pub visible: bool,
    /// Whether the overlay is modal.
    pub modal: bool,
    /// Title text for the progress surface.
    pub title: String,
    /// Optional detail line.
    pub detail: Option<String>,
    /// Completed steps.
    pub completed: usize,
    /// Total steps.
    pub total: usize,
    /// Whether the running operation supports cancel.
    pub cancelable: bool,
    /// Whether cancel has already been requested.
    pub cancel_requested: bool,
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

/// Modal confirmation prompt projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConfirmPromptModel {
    /// Whether the prompt is currently visible.
    pub visible: bool,
    /// Prompt kind used by the bridge to resolve confirm/cancel behavior.
    pub kind: Option<ConfirmPromptKind>,
    /// Prompt title text.
    pub title: String,
    /// Prompt body text.
    pub message: String,
    /// Confirm action label.
    pub confirm_label: String,
    /// Cancel action label.
    pub cancel_label: String,
    /// Optional target label shown as supplemental metadata.
    pub target_label: Option<String>,
    /// Optional editable prompt input value.
    pub input_value: Option<String>,
    /// Placeholder text for editable prompt input fields.
    pub input_placeholder: Option<String>,
    /// Optional validation error shown below editable prompt input.
    pub input_error: Option<String>,
}

/// Drag/drop overlay content for native-shell feedback.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DragOverlayModel {
    /// Whether a drag payload is currently active.
    pub active: bool,
    /// Human-friendly payload label.
    pub label: String,
    /// Current hover target label.
    pub target_label: String,
    /// Whether the current target is a valid drop.
    pub valid_target: bool,
    /// Cursor anchor x-coordinate for the floating drag chip, when available.
    pub pointer_x: Option<u16>,
    /// Cursor anchor y-coordinate for the floating drag chip, when available.
    pub pointer_y: Option<u16>,
}

/// Normalized waveform range with deterministic milli, micro, and nano projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRangeModel {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
    /// Start position in normalized micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// End position in normalized micro-units (`0..=1_000_000`).
    pub end_micros: u32,
    /// Start position in normalized nanounits (`0..=1_000_000_000`).
    pub start_nanos: u32,
    /// End position in normalized nanounits (`0..=1_000_000_000`).
    pub end_nanos: u32,
}

impl NormalizedRangeModel {
    /// Build a normalized range, clamping bounds to `0..=1000` and ordering them.
    pub fn new(start_milli: u16, end_milli: u16) -> Self {
        Self::from_micros(
            u32::from(start_milli.min(1000)) * 1000,
            u32::from(end_milli.min(1000)) * 1000,
        )
    }

    /// Build a normalized range from micro precision while preserving ordered milli mirrors.
    pub fn from_micros(start_micros: u32, end_micros: u32) -> Self {
        Self::from_nanos(
            start_micros.min(1_000_000).saturating_mul(1000),
            end_micros.min(1_000_000).saturating_mul(1000),
        )
    }

    /// Build a normalized range from nano precision while preserving ordered mirrors.
    pub fn from_nanos(start_nanos: u32, end_nanos: u32) -> Self {
        let start = start_nanos.min(1_000_000_000);
        let end = end_nanos.min(1_000_000_000);
        let ordered_start = start.min(end);
        let ordered_end = end.max(start);
        Self {
            start_milli: nanos_to_milli(ordered_start),
            end_milli: nanos_to_milli(ordered_end),
            start_micros: nanos_to_micros(ordered_start),
            end_micros: nanos_to_micros(ordered_end),
            start_nanos: ordered_start,
            end_nanos: ordered_end,
        }
    }
}

/// One detected waveform slice preview exposed to the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformSlicePreviewModel {
    /// Detected slice range in normalized milli, micro, and nano precision.
    pub range: NormalizedRangeModel,
    /// Whether this slice is currently selected for slice-edit operations.
    pub selected: bool,
    /// Whether this slice is focused for keyboard review audition.
    pub focused: bool,
    /// Whether this slice is marked for export.
    pub marked_for_export: bool,
    /// Whether this slice belongs to a duplicate-cleanup preview batch.
    pub duplicate_cleanup_candidate: bool,
    /// Whether this duplicate preview is currently exempted from cleanup.
    pub duplicate_cleanup_exempted: bool,
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

/// Waveform channel-view mode used by waveform rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformChannelViewModel {
    /// Collapse channels into one mono envelope.
    Mono,
    /// Render left/right channels in split stereo mode.
    Stereo,
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

/// Stable identifier for one of the two fixed folder panes in the sidebar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FolderPaneIdModel {
    /// Upper folder pane shown directly beneath the shared sources list.
    #[default]
    Upper,
    /// Lower folder pane shown beneath the upper pane.
    Lower,
}

impl FolderPaneIdModel {
    /// Return the small stable identifier used by automation and routing.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
        }
    }
}

/// Render data for one triage/browser column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnModel {
    /// Display label for the column header.
    pub title: String,
    /// Number of rows/items represented by the column.
    pub item_count: usize,
}

impl ColumnModel {
    /// Build a new column model.
    pub fn new(title: impl Into<String>, item_count: usize) -> Self {
        Self {
            title: title.into(),
            item_count,
        }
    }
}

/// Render data for one source row shown in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRowModel {
    /// Primary label shown for the source.
    pub label: String,
    /// Optional secondary detail text, usually a path or status.
    pub detail: String,
    /// Whether the row is currently selected.
    pub selected: bool,
    /// Whether the source is missing from disk.
    pub missing: bool,
    /// Whether this source is assigned to the upper folder pane.
    pub assigned_to_upper_pane: bool,
    /// Whether this source is assigned to the lower folder pane.
    pub assigned_to_lower_pane: bool,
}

impl SourceRowModel {
    /// Build a new source row model.
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        selected: bool,
        missing: bool,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            selected,
            missing,
            assigned_to_upper_pane: false,
            assigned_to_lower_pane: false,
        }
    }

    /// Mark whether this source is assigned to either fixed folder pane.
    pub fn with_pane_assignment(mut self, upper: bool, lower: bool) -> Self {
        self.assigned_to_upper_pane = upper;
        self.assigned_to_lower_pane = lower;
        self
    }
}

/// Render data for one folder row shown in the sidebar folder tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FolderRowKind {
    /// Standard existing folder row projected from host state.
    #[default]
    Existing,
    /// Inline draft row used while creating a new folder in place.
    CreateDraft,
    /// Inline draft row used while renaming an existing folder in place.
    RenameDraft,
}

/// Render data for one folder row shown in the sidebar folder tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderRowModel {
    /// Display label for the folder row.
    pub label: String,
    /// Optional secondary detail text for the folder row.
    pub detail: String,
    /// Tree depth used for indentation.
    pub depth: usize,
    /// Whether this row is currently selected.
    pub selected: bool,
    /// Whether this row currently has keyboard focus.
    pub focused: bool,
    /// Whether this row represents the synthetic source root.
    pub is_root: bool,
    /// Whether this row has child folders.
    pub has_children: bool,
    /// Whether this row is expanded in the folder tree.
    pub expanded: bool,
    /// Row kind used by the shell for inline draft rendering and hit testing.
    pub kind: FolderRowKind,
    /// Source/controller row index backing this projected row, when applicable.
    pub source_index: Option<usize>,
    /// Editable input value for inline draft rows.
    pub input_value: Option<String>,
    /// Placeholder text for inline draft rows.
    pub input_placeholder: Option<String>,
    /// Validation error for inline draft rows.
    pub input_error: Option<String>,
    /// Whether the inline draft input should own keyboard focus.
    pub input_focused: bool,
    /// Whether the next focus transition should select the full input text once.
    pub select_all_on_focus: bool,
}

impl FolderRowModel {
    /// Build a new folder row model.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        depth: usize,
        selected: bool,
        focused: bool,
        is_root: bool,
        has_children: bool,
        expanded: bool,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            depth,
            selected,
            focused,
            is_root,
            has_children,
            expanded,
            kind: FolderRowKind::Existing,
            source_index: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
            input_focused: false,
            select_all_on_focus: false,
        }
    }

    /// Attach the backing source/controller row index for one existing row.
    pub fn with_source_index(mut self, source_index: usize) -> Self {
        self.source_index = Some(source_index);
        self
    }

    /// Build one inline create-draft row embedded in the folder tree.
    pub fn create_draft(
        depth: usize,
        input_value: impl Into<String>,
        input_placeholder: impl Into<String>,
        input_error: Option<String>,
        input_focused: bool,
    ) -> Self {
        Self {
            label: String::new(),
            detail: String::new(),
            depth,
            selected: false,
            focused: false,
            is_root: false,
            has_children: false,
            expanded: false,
            kind: FolderRowKind::CreateDraft,
            source_index: None,
            input_value: Some(input_value.into()),
            input_placeholder: Some(input_placeholder.into()),
            input_error,
            input_focused,
            select_all_on_focus: false,
        }
    }

    /// Build one inline rename-draft row embedded in the folder tree.
    pub fn rename_draft(
        depth: usize,
        input_value: impl Into<String>,
        input_placeholder: impl Into<String>,
        input_error: Option<String>,
        input_focused: bool,
    ) -> Self {
        let input_value = input_value.into();
        Self {
            label: input_value.clone(),
            detail: String::new(),
            depth,
            selected: false,
            focused: false,
            is_root: false,
            has_children: false,
            expanded: false,
            kind: FolderRowKind::RenameDraft,
            source_index: None,
            input_value: Some(input_value),
            input_placeholder: Some(input_placeholder.into()),
            input_error,
            input_focused,
            select_all_on_focus: true,
        }
    }

    /// Set whether the inline input should select all text the next time it receives focus.
    pub fn with_select_all_on_focus(mut self, select_all_on_focus: bool) -> Self {
        self.select_all_on_focus = select_all_on_focus;
        self
    }
}

/// Native folder-action availability consumed by sidebar action surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FolderActionsModel {
    /// Whether creating a folder at the focused parent is allowed.
    pub can_create_folder: bool,
    /// Whether creating a folder at source root is allowed.
    pub can_create_folder_at_root: bool,
    /// Whether renaming the focused folder is allowed.
    pub can_rename_folder: bool,
    /// Whether deleting the focused folder is allowed.
    pub can_delete_folder: bool,
    /// Whether explicit restore for retained folder deletes is allowed.
    pub can_restore_retained_deletes: bool,
    /// Whether explicit purge for retained folder deletes is allowed.
    pub can_purge_retained_deletes: bool,
    /// Whether clearing folder delete-recovery logs is allowed.
    pub can_clear_recovery_log: bool,
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

/// Delete-recovery status for staged folder delete recovery in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FolderRecoveryModel {
    /// Whether delete recovery is still running in the background.
    pub in_progress: bool,
    /// Number of completed recovery log entries currently visible.
    pub entry_count: usize,
    /// Number of retained deletes currently awaiting explicit restore or purge.
    pub retained_count: usize,
}

/// Projected data for one fixed folder pane shown in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FolderPaneModel {
    /// Stable pane identity used by native routing.
    pub pane: FolderPaneIdModel,
    /// Short title shown in the pane header.
    pub title: String,
    /// Primary source label currently assigned to the pane.
    pub source_label: String,
    /// Secondary source detail text, usually the source path.
    pub source_detail: String,
    /// Whether this pane currently drives browser and waveform state.
    pub active: bool,
    /// Whether a source is assigned to this pane.
    pub has_source: bool,
    /// Whether this pane is hydrating its assigned source snapshot.
    pub loading: bool,
    /// Whether this pane is asynchronously rebuilding its folder-tree rows.
    pub projecting: bool,
    /// Whether this pane's source currently owns a background file or folder mutation.
    pub mutation_busy: bool,
    /// Active folder-search query for this pane.
    pub folder_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_folders: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_folders: bool,
    /// Whether folder filtering includes descendant files in a flattened list.
    pub flattened_view: bool,
    /// Whether the folder flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
    /// Focused folder row index, if any.
    pub focused_folder_row: Option<usize>,
    /// Folder rows to render in this pane.
    pub folder_rows: RetainedVec<FolderRowModel>,
    /// Folder action availability projected for this pane.
    pub folder_actions: FolderActionsModel,
    /// Folder delete-recovery summary projected for this pane.
    pub folder_recovery: FolderRecoveryModel,
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
    pub folder_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_folders: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_folders: bool,
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
    pub focused_folder_row: Option<usize>,
    /// Rows to render in the source panel.
    pub rows: RetainedVec<SourceRowModel>,
    /// Folder rows to render in the folder browser section.
    pub folder_rows: RetainedVec<FolderRowModel>,
    /// Folder action availability for native sidebar controls.
    pub folder_actions: FolderActionsModel,
    /// Folder delete-recovery summary for native sidebar status.
    pub folder_recovery: FolderRecoveryModel,
}

impl SourcesPanelModel {
    /// Borrow one pane model by id.
    pub fn folder_pane(&self, pane: FolderPaneIdModel) -> &FolderPaneModel {
        match pane {
            FolderPaneIdModel::Upper => &self.upper_folder_pane,
            FolderPaneIdModel::Lower => &self.lower_folder_pane,
        }
    }

    /// Borrow the pane that currently drives browser and waveform state.
    pub fn active_folder_pane_model(&self) -> &FolderPaneModel {
        self.folder_pane(self.active_folder_pane)
    }
}

fn micros_to_milli(value_micros: u32) -> u16 {
    ((value_micros.min(1_000_000) + 500) / 1000) as u16
}

fn nanos_to_micros(value_nanos: u32) -> u32 {
    ((value_nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000)
}

fn nanos_to_milli(value_nanos: u32) -> u16 {
    micros_to_milli(nanos_to_micros(value_nanos))
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
        Self::from(compat::AppModel::default())
    }
}

impl<T, U> From<compat::RetainedVec<T>> for RetainedVec<U>
where
    T: Clone + Into<U>,
{
    fn from(value: compat::RetainedVec<T>) -> Self {
        value
            .as_slice()
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<_>>()
            .into()
    }
}

impl<T, U> From<RetainedVec<T>> for compat::RetainedVec<U>
where
    T: Clone + Into<U>,
{
    fn from(value: RetainedVec<T>) -> Self {
        value
            .as_slice()
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<_>>()
            .into()
    }
}

impl From<compat::FolderPaneIdModel> for FolderPaneIdModel {
    fn from(value: compat::FolderPaneIdModel) -> Self {
        match value {
            compat::FolderPaneIdModel::Upper => Self::Upper,
            compat::FolderPaneIdModel::Lower => Self::Lower,
        }
    }
}

impl From<FolderPaneIdModel> for compat::FolderPaneIdModel {
    fn from(value: FolderPaneIdModel) -> Self {
        match value {
            FolderPaneIdModel::Upper => Self::Upper,
            FolderPaneIdModel::Lower => Self::Lower,
        }
    }
}

impl From<compat::ColumnModel> for ColumnModel {
    fn from(value: compat::ColumnModel) -> Self {
        Self {
            title: value.title,
            item_count: value.item_count,
        }
    }
}

impl From<ColumnModel> for compat::ColumnModel {
    fn from(value: ColumnModel) -> Self {
        Self {
            title: value.title,
            item_count: value.item_count,
        }
    }
}

impl From<compat::SourceRowModel> for SourceRowModel {
    fn from(value: compat::SourceRowModel) -> Self {
        Self {
            label: value.label,
            detail: value.detail,
            selected: value.selected,
            missing: value.missing,
            assigned_to_upper_pane: value.assigned_to_upper_pane,
            assigned_to_lower_pane: value.assigned_to_lower_pane,
        }
    }
}

impl From<SourceRowModel> for compat::SourceRowModel {
    fn from(value: SourceRowModel) -> Self {
        Self {
            label: value.label,
            detail: value.detail,
            selected: value.selected,
            missing: value.missing,
            assigned_to_upper_pane: value.assigned_to_upper_pane,
            assigned_to_lower_pane: value.assigned_to_lower_pane,
        }
    }
}

impl From<compat::FolderRowKind> for FolderRowKind {
    fn from(value: compat::FolderRowKind) -> Self {
        match value {
            compat::FolderRowKind::Existing => Self::Existing,
            compat::FolderRowKind::CreateDraft => Self::CreateDraft,
            compat::FolderRowKind::RenameDraft => Self::RenameDraft,
        }
    }
}

impl From<FolderRowKind> for compat::FolderRowKind {
    fn from(value: FolderRowKind) -> Self {
        match value {
            FolderRowKind::Existing => Self::Existing,
            FolderRowKind::CreateDraft => Self::CreateDraft,
            FolderRowKind::RenameDraft => Self::RenameDraft,
        }
    }
}

impl From<compat::FolderRowModel> for FolderRowModel {
    fn from(value: compat::FolderRowModel) -> Self {
        Self {
            label: value.label,
            detail: value.detail,
            depth: value.depth,
            selected: value.selected,
            focused: value.focused,
            is_root: value.is_root,
            has_children: value.has_children,
            expanded: value.expanded,
            kind: value.kind.into(),
            source_index: value.source_index,
            input_value: value.input_value,
            input_placeholder: value.input_placeholder,
            input_error: value.input_error,
            input_focused: value.input_focused,
            select_all_on_focus: value.select_all_on_focus,
        }
    }
}

impl From<FolderRowModel> for compat::FolderRowModel {
    fn from(value: FolderRowModel) -> Self {
        Self {
            label: value.label,
            detail: value.detail,
            depth: value.depth,
            selected: value.selected,
            focused: value.focused,
            is_root: value.is_root,
            has_children: value.has_children,
            expanded: value.expanded,
            kind: value.kind.into(),
            source_index: value.source_index,
            input_value: value.input_value,
            input_placeholder: value.input_placeholder,
            input_error: value.input_error,
            input_focused: value.input_focused,
            select_all_on_focus: value.select_all_on_focus,
        }
    }
}

impl From<compat::FolderActionsModel> for FolderActionsModel {
    fn from(value: compat::FolderActionsModel) -> Self {
        Self {
            can_create_folder: value.can_create_folder,
            can_create_folder_at_root: value.can_create_folder_at_root,
            can_rename_folder: value.can_rename_folder,
            can_delete_folder: value.can_delete_folder,
            can_restore_retained_deletes: value.can_restore_retained_deletes,
            can_purge_retained_deletes: value.can_purge_retained_deletes,
            can_clear_recovery_log: value.can_clear_recovery_log,
        }
    }
}

impl From<FolderActionsModel> for compat::FolderActionsModel {
    fn from(value: FolderActionsModel) -> Self {
        Self {
            can_create_folder: value.can_create_folder,
            can_create_folder_at_root: value.can_create_folder_at_root,
            can_rename_folder: value.can_rename_folder,
            can_delete_folder: value.can_delete_folder,
            can_restore_retained_deletes: value.can_restore_retained_deletes,
            can_purge_retained_deletes: value.can_purge_retained_deletes,
            can_clear_recovery_log: value.can_clear_recovery_log,
        }
    }
}

impl From<compat::FocusContextModel> for FocusContextModel {
    fn from(value: compat::FocusContextModel) -> Self {
        match value {
            compat::FocusContextModel::None => Self::None,
            compat::FocusContextModel::Waveform => Self::Waveform,
            compat::FocusContextModel::SampleBrowser => Self::SampleBrowser,
            compat::FocusContextModel::SourceFolders => Self::SourceFolders,
            compat::FocusContextModel::SourcesList => Self::SourcesList,
        }
    }
}

impl From<FocusContextModel> for compat::FocusContextModel {
    fn from(value: FocusContextModel) -> Self {
        match value {
            FocusContextModel::None => Self::None,
            FocusContextModel::Waveform => Self::Waveform,
            FocusContextModel::SampleBrowser => Self::SampleBrowser,
            FocusContextModel::SourceFolders => Self::SourceFolders,
            FocusContextModel::SourcesList => Self::SourcesList,
        }
    }
}

impl From<compat::FolderRecoveryModel> for FolderRecoveryModel {
    fn from(value: compat::FolderRecoveryModel) -> Self {
        Self {
            in_progress: value.in_progress,
            entry_count: value.entry_count,
            retained_count: value.retained_count,
        }
    }
}

impl From<FolderRecoveryModel> for compat::FolderRecoveryModel {
    fn from(value: FolderRecoveryModel) -> Self {
        Self {
            in_progress: value.in_progress,
            entry_count: value.entry_count,
            retained_count: value.retained_count,
        }
    }
}

impl From<compat::FolderPaneModel> for FolderPaneModel {
    fn from(value: compat::FolderPaneModel) -> Self {
        Self {
            pane: value.pane.into(),
            title: value.title,
            source_label: value.source_label,
            source_detail: value.source_detail,
            active: value.active,
            has_source: value.has_source,
            loading: value.loading,
            projecting: value.projecting,
            mutation_busy: value.mutation_busy,
            folder_search_query: value.folder_search_query,
            show_all_folders: value.show_all_folders,
            can_toggle_show_all_folders: value.can_toggle_show_all_folders,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            focused_folder_row: value.focused_folder_row,
            folder_rows: value.folder_rows.into(),
            folder_actions: value.folder_actions.into(),
            folder_recovery: value.folder_recovery.into(),
        }
    }
}

impl From<FolderPaneModel> for compat::FolderPaneModel {
    fn from(value: FolderPaneModel) -> Self {
        Self {
            pane: value.pane.into(),
            title: value.title,
            source_label: value.source_label,
            source_detail: value.source_detail,
            active: value.active,
            has_source: value.has_source,
            loading: value.loading,
            projecting: value.projecting,
            mutation_busy: value.mutation_busy,
            folder_search_query: value.folder_search_query,
            show_all_folders: value.show_all_folders,
            can_toggle_show_all_folders: value.can_toggle_show_all_folders,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            focused_folder_row: value.focused_folder_row,
            folder_rows: value.folder_rows.into(),
            folder_actions: value.folder_actions.into(),
            folder_recovery: value.folder_recovery.into(),
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
            folder_search_query: value.folder_search_query,
            show_all_folders: value.show_all_folders,
            can_toggle_show_all_folders: value.can_toggle_show_all_folders,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            selected_row: value.selected_row,
            loading_row: value.loading_row,
            mutation_busy_row: value.mutation_busy_row,
            focused_folder_row: value.focused_folder_row,
            rows: value.rows.into(),
            folder_rows: value.folder_rows.into(),
            folder_actions: value.folder_actions.into(),
            folder_recovery: value.folder_recovery.into(),
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
            folder_search_query: value.folder_search_query,
            show_all_folders: value.show_all_folders,
            can_toggle_show_all_folders: value.can_toggle_show_all_folders,
            flattened_view: value.flattened_view,
            can_toggle_flattened_view: value.can_toggle_flattened_view,
            selected_row: value.selected_row,
            loading_row: value.loading_row,
            mutation_busy_row: value.mutation_busy_row,
            focused_folder_row: value.focused_folder_row,
            rows: value.rows.into(),
            folder_rows: value.folder_rows.into(),
            folder_actions: value.folder_actions.into(),
            folder_recovery: value.folder_recovery.into(),
        }
    }
}

impl From<&SourcesPanelModel> for compat::SourcesPanelModel {
    fn from(value: &SourcesPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::PlaybackAgeFilterChip> for PlaybackAgeFilterChip {
    fn from(value: compat::PlaybackAgeFilterChip) -> Self {
        match value {
            compat::PlaybackAgeFilterChip::NeverPlayed => Self::NeverPlayed,
            compat::PlaybackAgeFilterChip::OlderThanMonth => Self::OlderThanMonth,
            compat::PlaybackAgeFilterChip::OlderThanWeek => Self::OlderThanWeek,
        }
    }
}

impl From<PlaybackAgeFilterChip> for compat::PlaybackAgeFilterChip {
    fn from(value: PlaybackAgeFilterChip) -> Self {
        match value {
            PlaybackAgeFilterChip::NeverPlayed => Self::NeverPlayed,
            PlaybackAgeFilterChip::OlderThanMonth => Self::OlderThanMonth,
            PlaybackAgeFilterChip::OlderThanWeek => Self::OlderThanWeek,
        }
    }
}

impl From<compat::PlaybackAgeBucket> for PlaybackAgeBucket {
    fn from(value: compat::PlaybackAgeBucket) -> Self {
        match value {
            compat::PlaybackAgeBucket::Fresh => Self::Fresh,
            compat::PlaybackAgeBucket::OlderThanWeek => Self::OlderThanWeek,
            compat::PlaybackAgeBucket::OlderThanMonth => Self::OlderThanMonth,
            compat::PlaybackAgeBucket::NeverPlayed => Self::NeverPlayed,
        }
    }
}

impl From<PlaybackAgeBucket> for compat::PlaybackAgeBucket {
    fn from(value: PlaybackAgeBucket) -> Self {
        match value {
            PlaybackAgeBucket::Fresh => Self::Fresh,
            PlaybackAgeBucket::OlderThanWeek => Self::OlderThanWeek,
            PlaybackAgeBucket::OlderThanMonth => Self::OlderThanMonth,
            PlaybackAgeBucket::NeverPlayed => Self::NeverPlayed,
        }
    }
}

impl From<compat::BrowserRowModel> for BrowserRowModel {
    fn from(value: compat::BrowserRowModel) -> Self {
        Self {
            visible_row: value.visible_row,
            label: value.label,
            column: value.column,
            rating_level: value.rating_level,
            playback_age_bucket: value.playback_age_bucket.into(),
            bucket_label: value.bucket_label,
            similarity_display_strength: value.similarity_display_strength,
            selected: value.selected,
            focused: value.focused,
            missing: value.missing,
            locked: value.locked,
            marked: value.marked,
        }
    }
}

impl From<BrowserRowModel> for compat::BrowserRowModel {
    fn from(value: BrowserRowModel) -> Self {
        Self {
            visible_row: value.visible_row,
            label: value.label,
            column: value.column,
            rating_level: value.rating_level,
            playback_age_bucket: value.playback_age_bucket.into(),
            bucket_label: value.bucket_label,
            similarity_display_strength: value.similarity_display_strength,
            selected: value.selected,
            focused: value.focused,
            missing: value.missing,
            locked: value.locked,
            marked: value.marked,
        }
    }
}

impl From<compat::BrowserPanelModel> for BrowserPanelModel {
    fn from(value: compat::BrowserPanelModel) -> Self {
        Self {
            visible_count: value.visible_count,
            selected_visible_row: value.selected_visible_row,
            autoscroll: value.autoscroll,
            view_start_row: value.view_start_row,
            selected_path_count: value.selected_path_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            source_loading: value.source_loading,
            metadata_pending: value.metadata_pending,
            file_op_pending: value.file_op_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_sample_label: value.focused_sample_label,
            tag_sidebar: value.tag_sidebar.into(),
            anchor_visible_row: value.anchor_visible_row,
            rows: value.rows.into(),
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
            selected_path_count: value.selected_path_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            source_loading: value.source_loading,
            metadata_pending: value.metadata_pending,
            file_op_pending: value.file_op_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_sample_label: value.focused_sample_label,
            tag_sidebar: value.tag_sidebar.into(),
            anchor_visible_row: value.anchor_visible_row,
            rows: value.rows.into(),
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
            samples_tab_label: value.samples_tab_label,
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
            samples_tab_label: value.samples_tab_label,
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
            can_tag: value.can_tag,
            can_normalize_focused_sample: value.can_normalize_focused_sample,
            can_loop_crossfade_focused_sample: value.can_loop_crossfade_focused_sample,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            tag_sidebar_open: value.tag_sidebar_open,
        }
    }
}

impl From<BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: BrowserActionsModel) -> Self {
        Self {
            can_rename: value.can_rename,
            can_delete: value.can_delete,
            can_tag: value.can_tag,
            can_normalize_focused_sample: value.can_normalize_focused_sample,
            can_loop_crossfade_focused_sample: value.can_loop_crossfade_focused_sample,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            tag_sidebar_open: value.tag_sidebar_open,
        }
    }
}

impl From<&BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: &BrowserActionsModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserTagState> for BrowserTagState {
    fn from(value: compat::BrowserTagState) -> Self {
        match value {
            compat::BrowserTagState::Off => Self::Off,
            compat::BrowserTagState::On => Self::On,
            compat::BrowserTagState::Mixed => Self::Mixed,
        }
    }
}

impl From<BrowserTagState> for compat::BrowserTagState {
    fn from(value: BrowserTagState) -> Self {
        match value {
            BrowserTagState::Off => Self::Off,
            BrowserTagState::On => Self::On,
            BrowserTagState::Mixed => Self::Mixed,
        }
    }
}

impl From<compat::BrowserTagPillModel> for BrowserTagPillModel {
    fn from(value: compat::BrowserTagPillModel) -> Self {
        Self {
            id: value.id,
            label: value.label,
            state: value.state.into(),
        }
    }
}

impl From<BrowserTagPillModel> for compat::BrowserTagPillModel {
    fn from(value: BrowserTagPillModel) -> Self {
        Self {
            id: value.id,
            label: value.label,
            state: value.state.into(),
        }
    }
}

impl From<compat::BrowserTagSidebarModel> for BrowserTagSidebarModel {
    fn from(value: compat::BrowserTagSidebarModel) -> Self {
        Self {
            open: value.open,
            selected_count: value.selected_count,
            header_label: value.header_label,
            auto_rename_enabled: value.auto_rename_enabled,
            input_value: value.input_value,
            input_placeholder: value.input_placeholder,
            playback_type_pills: value.playback_type_pills.map(Into::into),
            sound_type_pills: value.sound_type_pills.into_iter().map(Into::into).collect(),
            custom_tag_pill: value.custom_tag_pill.map(Into::into),
        }
    }
}

impl From<BrowserTagSidebarModel> for compat::BrowserTagSidebarModel {
    fn from(value: BrowserTagSidebarModel) -> Self {
        Self {
            open: value.open,
            selected_count: value.selected_count,
            header_label: value.header_label,
            auto_rename_enabled: value.auto_rename_enabled,
            input_value: value.input_value,
            input_placeholder: value.input_placeholder,
            playback_type_pills: value.playback_type_pills.map(Into::into),
            sound_type_pills: value.sound_type_pills.into_iter().map(Into::into).collect(),
            custom_tag_pill: value.custom_tag_pill.map(Into::into),
        }
    }
}

impl From<&BrowserTagSidebarModel> for compat::BrowserTagSidebarModel {
    fn from(value: &BrowserTagSidebarModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::MapRenderModeModel> for MapRenderModeModel {
    fn from(value: compat::MapRenderModeModel) -> Self {
        match value {
            compat::MapRenderModeModel::Heatmap => Self::Heatmap,
            compat::MapRenderModeModel::Points => Self::Points,
        }
    }
}

impl From<MapRenderModeModel> for compat::MapRenderModeModel {
    fn from(value: MapRenderModeModel) -> Self {
        match value {
            MapRenderModeModel::Heatmap => Self::Heatmap,
            MapRenderModeModel::Points => Self::Points,
        }
    }
}

impl From<compat::MapPointModel> for MapPointModel {
    fn from(value: compat::MapPointModel) -> Self {
        Self {
            sample_id: value.sample_id,
            x_milli: value.x_milli,
            y_milli: value.y_milli,
            cluster_id: value.cluster_id,
        }
    }
}

impl From<MapPointModel> for compat::MapPointModel {
    fn from(value: MapPointModel) -> Self {
        Self {
            sample_id: value.sample_id,
            x_milli: value.x_milli,
            y_milli: value.y_milli,
            cluster_id: value.cluster_id,
        }
    }
}

impl From<compat::MapPanelModel> for MapPanelModel {
    fn from(value: compat::MapPanelModel) -> Self {
        Self {
            active: value.active,
            summary: value.summary,
            legend_label: value.legend_label,
            selection_label: value.selection_label,
            hover_label: value.hover_label,
            cluster_label: value.cluster_label,
            viewport_label: value.viewport_label,
            error: value.error,
            render_mode: value.render_mode.into(),
            selected_sample_id: value.selected_sample_id,
            focused_sample_id: value.focused_sample_id,
            points: value
                .points
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into(),
        }
    }
}

impl From<MapPanelModel> for compat::MapPanelModel {
    fn from(value: MapPanelModel) -> Self {
        Self {
            active: value.active,
            summary: value.summary,
            legend_label: value.legend_label,
            selection_label: value.selection_label,
            hover_label: value.hover_label,
            cluster_label: value.cluster_label,
            viewport_label: value.viewport_label,
            error: value.error,
            render_mode: value.render_mode.into(),
            selected_sample_id: value.selected_sample_id,
            focused_sample_id: value.focused_sample_id,
            points: value
                .points
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into(),
        }
    }
}

impl From<&MapPanelModel> for compat::MapPanelModel {
    fn from(value: &MapPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::StatusBarModel> for StatusBarModel {
    fn from(value: compat::StatusBarModel) -> Self {
        Self {
            left: value.left,
            center: value.center,
            right: value.right,
        }
    }
}

impl From<StatusBarModel> for compat::StatusBarModel {
    fn from(value: StatusBarModel) -> Self {
        Self {
            left: value.left,
            center: value.center,
            right: value.right,
        }
    }
}

impl From<&StatusBarModel> for compat::StatusBarModel {
    fn from(value: &StatusBarModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::AudioEngineChipStateModel> for AudioEngineChipStateModel {
    fn from(value: compat::AudioEngineChipStateModel) -> Self {
        match value {
            compat::AudioEngineChipStateModel::Healthy => Self::Healthy,
            compat::AudioEngineChipStateModel::Error => Self::Error,
        }
    }
}

impl From<AudioEngineChipStateModel> for compat::AudioEngineChipStateModel {
    fn from(value: AudioEngineChipStateModel) -> Self {
        match value {
            AudioEngineChipStateModel::Healthy => Self::Healthy,
            AudioEngineChipStateModel::Error => Self::Error,
        }
    }
}

impl From<compat::AudioPickerTargetModel> for AudioPickerTargetModel {
    fn from(value: compat::AudioPickerTargetModel) -> Self {
        match value {
            compat::AudioPickerTargetModel::OutputHost => Self::OutputHost,
            compat::AudioPickerTargetModel::OutputDevice => Self::OutputDevice,
            compat::AudioPickerTargetModel::OutputSampleRate => Self::OutputSampleRate,
            compat::AudioPickerTargetModel::InputHost => Self::InputHost,
            compat::AudioPickerTargetModel::InputDevice => Self::InputDevice,
            compat::AudioPickerTargetModel::InputSampleRate => Self::InputSampleRate,
        }
    }
}

impl From<AudioPickerTargetModel> for compat::AudioPickerTargetModel {
    fn from(value: AudioPickerTargetModel) -> Self {
        match value {
            AudioPickerTargetModel::OutputHost => Self::OutputHost,
            AudioPickerTargetModel::OutputDevice => Self::OutputDevice,
            AudioPickerTargetModel::OutputSampleRate => Self::OutputSampleRate,
            AudioPickerTargetModel::InputHost => Self::InputHost,
            AudioPickerTargetModel::InputDevice => Self::InputDevice,
            AudioPickerTargetModel::InputSampleRate => Self::InputSampleRate,
        }
    }
}

impl From<compat::AudioOptionValueModel> for AudioOptionValueModel {
    fn from(value: compat::AudioOptionValueModel) -> Self {
        match value {
            compat::AudioOptionValueModel::OutputHost(value) => Self::OutputHost(value),
            compat::AudioOptionValueModel::OutputDevice(value) => Self::OutputDevice(value),
            compat::AudioOptionValueModel::OutputSampleRate(value) => Self::OutputSampleRate(value),
            compat::AudioOptionValueModel::InputHost(value) => Self::InputHost(value),
            compat::AudioOptionValueModel::InputDevice(value) => Self::InputDevice(value),
            compat::AudioOptionValueModel::InputSampleRate(value) => Self::InputSampleRate(value),
        }
    }
}

impl From<AudioOptionValueModel> for compat::AudioOptionValueModel {
    fn from(value: AudioOptionValueModel) -> Self {
        match value {
            AudioOptionValueModel::OutputHost(value) => Self::OutputHost(value),
            AudioOptionValueModel::OutputDevice(value) => Self::OutputDevice(value),
            AudioOptionValueModel::OutputSampleRate(value) => Self::OutputSampleRate(value),
            AudioOptionValueModel::InputHost(value) => Self::InputHost(value),
            AudioOptionValueModel::InputDevice(value) => Self::InputDevice(value),
            AudioOptionValueModel::InputSampleRate(value) => Self::InputSampleRate(value),
        }
    }
}

impl From<compat::AudioOptionItemModel> for AudioOptionItemModel {
    fn from(value: compat::AudioOptionItemModel) -> Self {
        Self {
            label: value.label,
            selected: value.selected,
            value: value.value.into(),
        }
    }
}

impl From<AudioOptionItemModel> for compat::AudioOptionItemModel {
    fn from(value: AudioOptionItemModel) -> Self {
        Self {
            label: value.label,
            selected: value.selected,
            value: value.value.into(),
        }
    }
}

impl From<compat::AudioFieldModel> for AudioFieldModel {
    fn from(value: compat::AudioFieldModel) -> Self {
        Self {
            label: value.label,
            value_label: value.value_label,
        }
    }
}

impl From<AudioFieldModel> for compat::AudioFieldModel {
    fn from(value: AudioFieldModel) -> Self {
        Self {
            label: value.label,
            value_label: value.value_label,
        }
    }
}

impl From<compat::AudioEngineModel> for AudioEngineModel {
    fn from(value: compat::AudioEngineModel) -> Self {
        Self {
            chip_state: value.chip_state.into(),
            chip_label: value.chip_label,
            detail_label: value.detail_label,
            output_host: value.output_host.into(),
            output_device: value.output_device.into(),
            output_sample_rate: value.output_sample_rate.into(),
            input_host: value.input_host.into(),
            input_device: value.input_device.into(),
            input_sample_rate: value.input_sample_rate.into(),
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .output_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_device_options: value
                .output_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_sample_rate_options: value
                .output_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_host_options: value
                .input_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_device_options: value
                .input_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_sample_rate_options: value
                .input_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<AudioEngineModel> for compat::AudioEngineModel {
    fn from(value: AudioEngineModel) -> Self {
        Self {
            chip_state: value.chip_state.into(),
            chip_label: value.chip_label,
            detail_label: value.detail_label,
            output_host: value.output_host.into(),
            output_device: value.output_device.into(),
            output_sample_rate: value.output_sample_rate.into(),
            input_host: value.input_host.into(),
            input_device: value.input_device.into(),
            input_sample_rate: value.input_sample_rate.into(),
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .output_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_device_options: value
                .output_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            output_sample_rate_options: value
                .output_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_host_options: value
                .input_host_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_device_options: value
                .input_device_options
                .into_iter()
                .map(Into::into)
                .collect(),
            input_sample_rate_options: value
                .input_sample_rate_options
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<&AudioEngineModel> for compat::AudioEngineModel {
    fn from(value: &AudioEngineModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::UpdateStatusModel> for UpdateStatusModel {
    fn from(value: compat::UpdateStatusModel) -> Self {
        match value {
            compat::UpdateStatusModel::Idle => Self::Idle,
            compat::UpdateStatusModel::Checking => Self::Checking,
            compat::UpdateStatusModel::Available => Self::Available,
            compat::UpdateStatusModel::Error => Self::Error,
        }
    }
}

impl From<UpdateStatusModel> for compat::UpdateStatusModel {
    fn from(value: UpdateStatusModel) -> Self {
        match value {
            UpdateStatusModel::Idle => Self::Idle,
            UpdateStatusModel::Checking => Self::Checking,
            UpdateStatusModel::Available => Self::Available,
            UpdateStatusModel::Error => Self::Error,
        }
    }
}

impl From<compat::UpdatePanelModel> for UpdatePanelModel {
    fn from(value: compat::UpdatePanelModel) -> Self {
        Self {
            status: value.status.into(),
            status_label: value.status_label,
            action_hint_label: value.action_hint_label,
            release_notes_label: value.release_notes_label,
            available_tag: value.available_tag,
            available_url: value.available_url,
            last_error: value.last_error,
        }
    }
}

impl From<UpdatePanelModel> for compat::UpdatePanelModel {
    fn from(value: UpdatePanelModel) -> Self {
        Self {
            status: value.status.into(),
            status_label: value.status_label,
            action_hint_label: value.action_hint_label,
            release_notes_label: value.release_notes_label,
            available_tag: value.available_tag,
            available_url: value.available_url,
            last_error: value.last_error,
        }
    }
}

impl From<&UpdatePanelModel> for compat::UpdatePanelModel {
    fn from(value: &UpdatePanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::ProgressOverlayModel> for ProgressOverlayModel {
    fn from(value: compat::ProgressOverlayModel) -> Self {
        Self {
            visible: value.visible,
            modal: value.modal,
            title: value.title,
            detail: value.detail,
            completed: value.completed,
            total: value.total,
            cancelable: value.cancelable,
            cancel_requested: value.cancel_requested,
        }
    }
}

impl From<ProgressOverlayModel> for compat::ProgressOverlayModel {
    fn from(value: ProgressOverlayModel) -> Self {
        Self {
            visible: value.visible,
            modal: value.modal,
            title: value.title,
            detail: value.detail,
            completed: value.completed,
            total: value.total,
            cancelable: value.cancelable,
            cancel_requested: value.cancel_requested,
        }
    }
}

impl From<&ProgressOverlayModel> for compat::ProgressOverlayModel {
    fn from(value: &ProgressOverlayModel) -> Self {
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
            compat::ConfirmPromptKind::DestructiveEdit => Self::DestructiveEdit,
            compat::ConfirmPromptKind::BrowserRename => Self::BrowserRename,
            compat::ConfirmPromptKind::FolderRename => Self::FolderRename,
            compat::ConfirmPromptKind::FolderCreate => Self::FolderCreate,
            compat::ConfirmPromptKind::RestoreRetainedFolderDeletes => {
                Self::RestoreRetainedFolderDeletes
            }
            compat::ConfirmPromptKind::PurgeRetainedFolderDeletes => {
                Self::PurgeRetainedFolderDeletes
            }
            compat::ConfirmPromptKind::OptionsDefaultIdentifier => Self::OptionsDefaultIdentifier,
        }
    }
}

impl From<ConfirmPromptKind> for compat::ConfirmPromptKind {
    fn from(value: ConfirmPromptKind) -> Self {
        match value {
            ConfirmPromptKind::DestructiveEdit => Self::DestructiveEdit,
            ConfirmPromptKind::BrowserRename => Self::BrowserRename,
            ConfirmPromptKind::FolderRename => Self::FolderRename,
            ConfirmPromptKind::FolderCreate => Self::FolderCreate,
            ConfirmPromptKind::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            ConfirmPromptKind::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            ConfirmPromptKind::OptionsDefaultIdentifier => Self::OptionsDefaultIdentifier,
        }
    }
}

impl From<compat::ConfirmPromptModel> for ConfirmPromptModel {
    fn from(value: compat::ConfirmPromptModel) -> Self {
        Self {
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
}

impl From<ConfirmPromptModel> for compat::ConfirmPromptModel {
    fn from(value: ConfirmPromptModel) -> Self {
        Self {
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
}

impl From<&ConfirmPromptModel> for compat::ConfirmPromptModel {
    fn from(value: &ConfirmPromptModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::DragOverlayModel> for DragOverlayModel {
    fn from(value: compat::DragOverlayModel) -> Self {
        Self {
            active: value.active,
            label: value.label,
            target_label: value.target_label,
            valid_target: value.valid_target,
            pointer_x: value.pointer_x,
            pointer_y: value.pointer_y,
        }
    }
}

impl From<DragOverlayModel> for compat::DragOverlayModel {
    fn from(value: DragOverlayModel) -> Self {
        Self {
            active: value.active,
            label: value.label,
            target_label: value.target_label,
            valid_target: value.valid_target,
            pointer_x: value.pointer_x,
            pointer_y: value.pointer_y,
        }
    }
}

impl From<&DragOverlayModel> for compat::DragOverlayModel {
    fn from(value: &DragOverlayModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::NormalizedRangeModel> for NormalizedRangeModel {
    fn from(value: compat::NormalizedRangeModel) -> Self {
        Self {
            start_milli: value.start_milli,
            end_milli: value.end_milli,
            start_micros: value.start_micros,
            end_micros: value.end_micros,
            start_nanos: value.start_nanos,
            end_nanos: value.end_nanos,
        }
    }
}

impl From<NormalizedRangeModel> for compat::NormalizedRangeModel {
    fn from(value: NormalizedRangeModel) -> Self {
        Self {
            start_milli: value.start_milli,
            end_milli: value.end_milli,
            start_micros: value.start_micros,
            end_micros: value.end_micros,
            start_nanos: value.start_nanos,
            end_nanos: value.end_nanos,
        }
    }
}

impl From<compat::WaveformSlicePreviewModel> for WaveformSlicePreviewModel {
    fn from(value: compat::WaveformSlicePreviewModel) -> Self {
        Self {
            range: value.range.into(),
            selected: value.selected,
            focused: value.focused,
            marked_for_export: value.marked_for_export,
            duplicate_cleanup_candidate: value.duplicate_cleanup_candidate,
            duplicate_cleanup_exempted: value.duplicate_cleanup_exempted,
        }
    }
}

impl From<WaveformSlicePreviewModel> for compat::WaveformSlicePreviewModel {
    fn from(value: WaveformSlicePreviewModel) -> Self {
        Self {
            range: value.range.into(),
            selected: value.selected,
            focused: value.focused,
            marked_for_export: value.marked_for_export,
            duplicate_cleanup_candidate: value.duplicate_cleanup_candidate,
            duplicate_cleanup_exempted: value.duplicate_cleanup_exempted,
        }
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

impl From<compat::WaveformChannelViewModel> for WaveformChannelViewModel {
    fn from(value: compat::WaveformChannelViewModel) -> Self {
        match value {
            compat::WaveformChannelViewModel::Mono => Self::Mono,
            compat::WaveformChannelViewModel::Stereo => Self::Stereo,
        }
    }
}

impl From<WaveformChannelViewModel> for compat::WaveformChannelViewModel {
    fn from(value: WaveformChannelViewModel) -> Self {
        match value {
            WaveformChannelViewModel::Mono => Self::Mono,
            WaveformChannelViewModel::Stereo => Self::Stereo,
        }
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
            audio_engine: value.audio_engine.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: value.confirm_prompt.into(),
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
            audio_engine: value.audio_engine.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel.into(),
            progress_overlay: value.progress_overlay.into(),
            confirm_prompt: value.confirm_prompt.into(),
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

#[cfg(test)]
mod tests {
    use super::{WaveformPanelModel, parse_waveform_tempo_number_text};

    #[test]
    fn waveform_panel_default_bpm_grid_origin_is_zero() {
        assert_eq!(WaveformPanelModel::default().bpm_grid_origin_micros, 0);
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
