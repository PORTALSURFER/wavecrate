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

/// Render data for one source row shown in the sidebar.
pub type SourceRowModel = panel::SplitPaneAssignedRow;

/// Transient browser row processing states for batch file operations.
pub type BrowserRowProcessingState = list::RowProcessingState;

/// Tri-state pill state used by the browser metadata editor.
pub type BrowserTagState = selection::TriState;

/// One clickable tag pill projected into the browser metadata sidebar.
pub type BrowserTagPillModel = badge::SelectablePill<BrowserTagState>;

/// Render mode label for the map panel.
pub type MapRenderModeModel = visualization::PointRenderMode;

/// Summary of map state consumed by the native shell map tab.
pub type MapPanelModel = visualization::SpatialPanel;

/// Channel-view mode used by waveform rendering.
pub type WaveformChannelViewModel = visualization::ChannelViewMode;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PlaybackAgeFilterChip {
    /// Samples that have never been played.
    NeverPlayed,
    /// Samples whose last playback was at least 30 days ago.
    OlderThanMonth,
    /// Samples whose last playback was at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

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
    bits: u16,
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

    const STATIC_MASK: u16 = Self::STATUS_BAR
        | Self::BROWSER_FRAME
        | Self::BROWSER_ROWS_WINDOW
        | Self::MAP_PANEL
        | Self::WAVEFORM_OVERLAY
        | Self::GLOBAL_STATIC;
    const OVERLAY_MASK: u16 = Self::STATE_OVERLAY | Self::MOTION_OVERLAY;

    /// Return an empty segment mask.
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Return a full segment mask.
    pub const fn all() -> Self {
        Self {
            bits: Self::STATIC_MASK | Self::OVERLAY_MASK,
        }
    }

    /// Construct a segment mask from raw bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self {
            bits: bits & (Self::STATIC_MASK | Self::OVERLAY_MASK),
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Return `true` when the mask contains no segments.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Return `true` when any static segment requires rebuild.
    pub const fn requires_static_rebuild(self) -> bool {
        (self.bits & Self::STATIC_MASK) != 0
    }

    /// Return `true` when any overlay segment requires rebuild.
    pub const fn requires_overlay_rebuild(self) -> bool {
        (self.bits & Self::OVERLAY_MASK) != 0
    }

    /// Insert one or more segment bits into this mask.
    pub fn insert(&mut self, bits: u16) {
        self.bits |= bits & (Self::STATIC_MASK | Self::OVERLAY_MASK);
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
    /// Return whether any static-segment revision is non-zero.
    pub const fn has_static_revisions(self) -> bool {
        self.status_bar != 0
            || self.browser_frame != 0
            || self.browser_rows_window != 0
            || self.map_panel != 0
            || self.waveform_overlay != 0
            || self.global_static != 0
    }

    /// Bump revisions for the static segments flagged in `dirty_segments`.
    pub fn bump_for_dirty_segments(&mut self, dirty_segments: DirtySegments) {
        let bits = dirty_segments.bits();
        if (bits & DirtySegments::STATUS_BAR) != 0 {
            self.status_bar = self.status_bar.saturating_add(1);
        }
        if (bits & DirtySegments::BROWSER_FRAME) != 0 {
            self.browser_frame = self.browser_frame.saturating_add(1);
        }
        if (bits & DirtySegments::BROWSER_ROWS_WINDOW) != 0 {
            self.browser_rows_window = self.browser_rows_window.saturating_add(1);
        }
        if (bits & DirtySegments::MAP_PANEL) != 0 {
            self.map_panel = self.map_panel.saturating_add(1);
        }
        if (bits & DirtySegments::WAVEFORM_OVERLAY) != 0 {
            self.waveform_overlay = self.waveform_overlay.saturating_add(1);
        }
        if (bits & DirtySegments::GLOBAL_STATIC) != 0 {
            self.global_static = self.global_static.saturating_add(1);
        }
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
        Self {
            transport_running: model.transport_running,
            map_active: model.map.active,
            active_rating_filters: model.browser.active_rating_filters,
            active_playback_age_filters: model.browser.active_playback_age_filters,
            marked_filter_active: model.browser.marked_filter_active,
            waveform_selection_milli: model.waveform.selection_milli,
            waveform_slices: model.waveform.slices.clone(),
            waveform_selection_export_flash_nonce: model.waveform.selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: model
                .waveform
                .selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: model
                .waveform
                .edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: model.waveform.edit_selection_milli,
            waveform_edit_fade_in_end_milli: model.waveform.edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: model.waveform.edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: model.waveform.edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: model.waveform.edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: model.waveform.edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: model.waveform.edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: model.waveform.edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: model.waveform.edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: model.waveform.edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: model.waveform.edit_fade_out_curve_milli,
            waveform_loop_enabled: model.waveform.loop_enabled,
            waveform_loop_lock_enabled: model.waveform_chrome.loop_lock_enabled,
            waveform_cursor_milli: model.waveform.cursor_milli,
            waveform_playhead_milli: model.waveform.playhead_milli,
            waveform_playhead_micros: model.waveform.playhead_micros.or_else(|| {
                model
                    .waveform
                    .playhead_milli
                    .map(|milli| u32::from(milli) * 1000)
            }),
            waveform_view_start_milli: model.waveform.view_start_milli,
            waveform_view_end_milli: model.waveform.view_end_milli,
            waveform_view_start_micros: model.waveform.view_start_micros,
            waveform_view_end_micros: model.waveform.view_end_micros,
            waveform_view_start_nanos: model.waveform.view_start_nanos,
            waveform_view_end_nanos: model.waveform.view_end_nanos,
            waveform_tempo_label: model.waveform.tempo_label.clone(),
            waveform_zoom_label: model.waveform.zoom_label.clone(),
            waveform_loaded_label: model.waveform.loaded_label.clone(),
            waveform_loading: model.waveform.loading,
            waveform_image_signature: model.waveform.waveform_image_signature,
            waveform_transport_hint: model.waveform_chrome.transport_hint.clone(),
            waveform_compare_anchor_available: model.waveform_chrome.compare_anchor_available,
            waveform_compare_anchor_label: model.waveform_chrome.compare_anchor_label.clone(),
            waveform_channel_view: model.waveform_chrome.channel_view,
            waveform_normalized_audition_enabled: model.waveform_chrome.normalized_audition_enabled,
            waveform_bpm_snap_enabled: model.waveform_chrome.bpm_snap_enabled,
            waveform_relative_bpm_grid_enabled: model.waveform_chrome.relative_bpm_grid_enabled,
            waveform_transient_snap_enabled: model.waveform_chrome.transient_snap_enabled,
            waveform_transient_markers_enabled: model.waveform_chrome.transient_markers_enabled,
            waveform_slice_mode_enabled: model.waveform_chrome.slice_mode_enabled,
            waveform_exact_duplicate_cleanup_available: model
                .waveform_chrome
                .exact_duplicate_cleanup_available,
            status_right: model.status.right.clone(),
        }
    }
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
    /// Transient row-scoped processing state for active batch operations.
    pub processing_state: BrowserRowProcessingState,
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
            processing_state: BrowserRowProcessingState::None,
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

    /// Attach a transient row-scoped processing state.
    pub fn with_processing_state(mut self, processing_state: BrowserRowProcessingState) -> Self {
        self.processing_state = processing_state;
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
    /// Current tag search/create input value.
    pub input_value: String,
    /// Placeholder shown for the tag input when empty.
    pub input_placeholder: String,
    /// Exclusive playback-type pills.
    pub playback_type_pills: [BrowserTagPillModel; 2],
    /// Normal tag candidates from common usage or search.
    pub normal_tag_pills: Vec<BrowserTagPillModel>,
    /// Create-new candidate when the input does not exactly match an existing tag.
    pub create_tag_pill: Option<BrowserTagPillModel>,
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
            folder_rows: retained_vec_from_compat(value.folder_rows),
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
            folder_rows: retained_vec_to_compat(value.folder_rows),
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
            rows: retained_vec_from_compat(value.rows),
            folder_rows: retained_vec_from_compat(value.folder_rows),
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
            rows: retained_vec_to_compat(value.rows),
            folder_rows: retained_vec_to_compat(value.folder_rows),
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
            processing_state: value.processing_state.into(),
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
            processing_state: value.processing_state.into(),
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
            tag_named_filter_active: value.tag_named_filter_active,
            tag_named_filter_negated: value.tag_named_filter_negated,
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
            selected_path_count: value.selected_path_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            tag_named_filter_active: value.tag_named_filter_active,
            tag_named_filter_negated: value.tag_named_filter_negated,
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
            normal_tag_pills: value.normal_tag_pills.into_iter().map(Into::into).collect(),
            create_tag_pill: value.create_tag_pill.map(Into::into),
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
            normal_tag_pills: value.normal_tag_pills.into_iter().map(Into::into).collect(),
            create_tag_pill: value.create_tag_pill.map(Into::into),
        }
    }
}

impl From<&BrowserTagSidebarModel> for compat::BrowserTagSidebarModel {
    fn from(value: &BrowserTagSidebarModel) -> Self {
        value.clone().into()
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

fn audio_option_item_from_compat(value: compat::AudioOptionItemModel) -> AudioOptionItemModel {
    AudioOptionItemModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

fn audio_option_item_to_compat(value: AudioOptionItemModel) -> compat::AudioOptionItemModel {
    compat::AudioOptionItemModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
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
                .map(audio_option_item_from_compat)
                .collect(),
            output_device_options: value
                .output_device_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_sample_rate_options: value
                .output_sample_rate_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_host_options: value
                .input_host_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_device_options: value
                .input_device_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_sample_rate_options: value
                .input_sample_rate_options
                .into_iter()
                .map(audio_option_item_from_compat)
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
                .map(audio_option_item_to_compat)
                .collect(),
            output_device_options: value
                .output_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            output_sample_rate_options: value
                .output_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            input_host_options: value
                .input_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            input_device_options: value
                .input_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            input_sample_rate_options: value
                .input_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
        }
    }
}

impl From<&AudioEngineModel> for compat::AudioEngineModel {
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
            audio_engine: value.audio_engine.into(),
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
    automation::AutomationNodeId(value.0)
}

fn automation_node_id_to_compat(value: AutomationNodeId) -> compat::AutomationNodeId {
    compat::AutomationNodeId(value.0)
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
            compat::AutomationRole::WaveformRegion => Self::WaveformRegion,
            compat::AutomationRole::MapCanvas => Self::MapCanvas,
            compat::AutomationRole::MapPoint => Self::MapPoint,
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
            AutomationRole::WaveformRegion => Self::WaveformRegion,
            AutomationRole::MapCanvas => Self::MapCanvas,
            AutomationRole::MapPoint => Self::MapPoint,
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
            available_actions: value.available_actions,
            metadata: value.metadata,
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
            available_actions: value.available_actions,
            metadata: value.metadata,
            children: value.children.into_iter().map(Into::into).collect(),
        }
    }
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
