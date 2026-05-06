//! Sempal-owned native shell projection DTOs.
//!
//! These models describe Sempal application state as projected for the current
//! native shell. The runtime adapter in `gui_runtime` converts these app-core
//! DTOs into the Sempal-owned native runtime contract consumed by Radiant.

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
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum BrowserRowProcessingState {
    /// The row is not part of an active row-scoped operation.
    #[default]
    None,
    /// The row is waiting in the current batch.
    Queued,
    /// The row is currently being processed.
    Active,
    /// The row completed successfully.
    Completed,
    /// The row was skipped by the batch.
    Skipped,
    /// The row failed during processing.
    Failed,
}

/// Summary of one Sempal browser/list row consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRowModel {
    /// Visible row index in the filtered browser list.
    pub visible_row: usize,
    /// Display label for the row.
    ///
    /// This text is reference-counted so retained app-model clones can reuse
    /// row payloads without copying every row label.
    pub label: Arc<str>,
    /// Triage or grouping column index that currently owns the row.
    pub column: usize,
    /// Signed row rating level shown alongside the row label (`-3..=3`).
    pub rating_level: i8,
    /// Visual playback-age bucket used to render the row age marker.
    pub playback_age_bucket: PlaybackAgeBucket,
    /// Optional inline metadata label rendered at the row edge.
    pub bucket_label: Option<Arc<str>>,
    /// Optional normalized relatedness fill amount encoded in the inclusive `0..=255` range.
    pub similarity_display_strength: Option<u8>,
    /// Whether this row is currently selected in multi-selection state.
    pub selected: bool,
    /// Whether this row currently has focus/caret.
    pub focused: bool,
    /// Whether the backing sample is unavailable.
    pub missing: bool,
    /// Whether the backing sample is locked/protected.
    pub locked: bool,
    /// Whether the backing sample is marked for later review.
    pub marked: bool,
    /// Transient row-scoped processing state for active batch file operations.
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

    /// Attach a signed rating level for inline row indicators.
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

    /// Attach a normalized relatedness display strength for a compact row bar.
    ///
    /// Values are clamped into `[0.0, 1.0]` and encoded into the integer-backed
    /// `similarity_display_strength` field so retained app-model snapshots can
    /// keep `Eq` semantics.
    pub fn with_similarity_display_strength(mut self, display_strength: f32) -> Self {
        self.similarity_display_strength =
            Some(Self::encode_similarity_display_strength(display_strength));
        self
    }

    /// Encode one normalized relatedness display strength into the stored byte range.
    pub fn encode_similarity_display_strength(display_strength: f32) -> u8 {
        (display_strength.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Decode the stored relatedness display strength into a normalized fill amount.
    pub fn similarity_display_strength_ratio(&self) -> Option<f32> {
        self.similarity_display_strength
            .map(|strength| f32::from(strength) / 255.0)
    }

    /// Mark whether the backing sample is unavailable.
    pub fn with_missing(mut self, missing: bool) -> Self {
        self.missing = missing;
        self
    }

    /// Mark whether the backing sample should render with protected treatment.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Mark whether the backing sample should render with review treatment.
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

/// Normalized waveform viewport state exposed to the native shell.
pub type WaveformViewportModel = visualization::TimelineViewport;

/// Waveform cursor, playhead, and selection transport state exposed to the native shell.
pub type WaveformTransportModel = visualization::TimelineTransportState;

/// Waveform edit selection and fade-preview state exposed to the native shell.
pub type WaveformEditPreviewModel = visualization::TimelineEditPreview;

/// One detected Sempal waveform slice preview exposed to the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformSlicePreviewModel {
    /// Slice range in normalized waveform precision.
    pub range: NormalizedRangeModel,
    /// Whether this slice is currently selected for edit operations.
    pub selected: bool,
    /// Whether this slice is focused for keyboard review.
    pub focused: bool,
    /// Whether this slice is marked for sample export.
    pub marked_for_export: bool,
    /// Whether this slice belongs to the duplicate-cleanup candidate batch.
    pub review_candidate: bool,
    /// Whether this slice is currently exempted from duplicate cleanup.
    pub review_exempted: bool,
}

/// One-shot waveform feedback event tokens exposed to the native shell.
pub type WaveformFeedbackEventsModel = visualization::TimelineFeedbackEvents;

/// Waveform guide/repeat/label presentation state exposed to the native shell.
pub type WaveformPresentationModel = visualization::TimelinePresentationState;

/// Retained waveform raster preview state exposed to the native shell.
pub type WaveformImagePreviewModel = visualization::SignalRasterPreview;

/// Waveform display chrome state exposed to the native shell.
pub type WaveformChromeStateModel = visualization::SignalChromeState;

/// Waveform tool availability state exposed to the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaveformToolStateModel {
    /// Whether loop playback is locked against sample-driven updates.
    pub lock_enabled: bool,
    /// Whether normalized audition playback is enabled.
    pub audition_enabled: bool,
    /// Whether BPM snapping is enabled.
    pub primary_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub relative_grid_enabled: bool,
    /// Whether transient snapping is enabled.
    pub secondary_snap_enabled: bool,
    /// Whether transient markers are visible.
    pub markers_visible: bool,
    /// Whether slice review mode is active.
    pub review_mode_enabled: bool,
    /// Whether exact-duplicate cleanup can be applied from the waveform toolbar.
    pub cleanup_available: bool,
}

impl Default for WaveformToolStateModel {
    fn default() -> Self {
        Self {
            lock_enabled: false,
            audition_enabled: false,
            primary_snap_enabled: false,
            relative_grid_enabled: false,
            secondary_snap_enabled: false,
            markers_visible: true,
            review_mode_enabled: false,
            cleanup_available: false,
        }
    }
}

impl WaveformToolStateModel {
    /// Build waveform tool state from explicit Sempal workflow flags.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        lock_enabled: bool,
        audition_enabled: bool,
        primary_snap_enabled: bool,
        relative_grid_enabled: bool,
        secondary_snap_enabled: bool,
        markers_visible: bool,
        review_mode_enabled: bool,
        cleanup_available: bool,
    ) -> Self {
        Self {
            lock_enabled,
            audition_enabled,
            primary_snap_enabled,
            relative_grid_enabled,
            secondary_snap_enabled,
            markers_visible,
            review_mode_enabled,
            cleanup_available,
        }
    }
}

/// Aggregated waveform timeline surface state exposed to the native shell.
pub type WaveformSurfaceModel = visualization::TimelineSurfaceState<WaveformSlicePreviewModel>;

/// Aggregated waveform motion state exposed to the native shell.
pub type WaveformMotionModel =
    visualization::TimelineMotionState<WaveformSlicePreviewModel, WaveformToolStateModel>;

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
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

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
            waveform_channel_view: signal_chrome.channel_view,
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

    /// Return this motion snapshot's generic timeline viewport state.
    pub fn waveform_viewport(&self) -> WaveformViewportModel {
        WaveformViewportModel::new(
            self.waveform_view_start_milli,
            self.waveform_view_end_milli,
            self.waveform_view_start_micros,
            self.waveform_view_end_micros,
            self.waveform_view_start_nanos,
            self.waveform_view_end_nanos,
        )
    }

    /// Return this motion snapshot's generic timeline transport state.
    pub fn waveform_transport(&self) -> WaveformTransportModel {
        WaveformTransportModel::new(
            self.waveform_cursor_milli,
            self.waveform_playhead_milli,
            self.waveform_playhead_micros,
            self.waveform_selection_milli,
        )
    }

    /// Return this motion snapshot's generic timeline edit-preview state.
    pub fn waveform_edit_preview(&self) -> WaveformEditPreviewModel {
        WaveformEditPreviewModel::new(
            self.waveform_edit_selection_milli,
            self.waveform_edit_fade_in_end_milli,
            self.waveform_edit_fade_in_end_micros,
            self.waveform_edit_fade_in_mute_start_milli,
            self.waveform_edit_fade_in_mute_start_micros,
            self.waveform_edit_fade_in_curve_milli,
            self.waveform_edit_fade_out_start_milli,
            self.waveform_edit_fade_out_start_micros,
            self.waveform_edit_fade_out_mute_end_milli,
            self.waveform_edit_fade_out_mute_end_micros,
            self.waveform_edit_fade_out_curve_milli,
        )
    }

    /// Return this motion snapshot's generic timeline feedback event tokens.
    pub fn waveform_feedback_events(&self) -> WaveformFeedbackEventsModel {
        WaveformFeedbackEventsModel::new(
            self.waveform_selection_export_flash_nonce,
            self.waveform_selection_export_failure_flash_nonce,
            self.waveform_edit_selection_apply_flash_nonce,
        )
    }

    /// Return this motion snapshot's generic timeline presentation state.
    pub fn waveform_presentation(&self) -> WaveformPresentationModel {
        WaveformPresentationModel::new(
            None,
            0,
            self.waveform_loop_enabled,
            self.waveform_tempo_label.clone(),
            self.waveform_zoom_label.clone(),
        )
    }

    /// Return this motion snapshot's generic retained raster preview state.
    pub fn waveform_image_preview(&self) -> WaveformImagePreviewModel {
        WaveformImagePreviewModel::new(
            self.waveform_loaded_label.clone(),
            self.waveform_loading,
            false,
            self.waveform_image_signature,
            None,
        )
    }

    /// Return this motion snapshot's generic signal chrome state.
    pub fn signal_chrome(&self) -> WaveformChromeStateModel {
        WaveformChromeStateModel::new(
            self.waveform_transport_hint.clone(),
            self.waveform_compare_anchor_available,
            self.waveform_compare_anchor_label.clone(),
            self.waveform_channel_view,
        )
    }

    /// Return this motion snapshot's generic signal tool state.
    pub fn signal_tools(&self) -> WaveformToolStateModel {
        WaveformToolStateModel::new(
            self.waveform_loop_lock_enabled,
            self.waveform_normalized_audition_enabled,
            self.waveform_bpm_snap_enabled,
            self.waveform_relative_bpm_grid_enabled,
            self.waveform_transient_snap_enabled,
            self.waveform_transient_markers_enabled,
            self.waveform_slice_mode_enabled,
            self.waveform_exact_duplicate_cleanup_available,
        )
    }

    /// Return this motion snapshot as a generic timeline motion aggregate.
    pub fn timeline_motion(&self) -> WaveformMotionModel {
        WaveformMotionModel::new(
            self.transport_running,
            WaveformSurfaceModel::new(
                self.waveform_viewport(),
                self.waveform_transport(),
                self.waveform_edit_preview(),
                self.waveform_feedback_events(),
                self.waveform_presentation(),
                self.waveform_image_preview(),
                self.waveform_slices.clone(),
            ),
            self.signal_chrome(),
            self.signal_tools(),
        )
    }
}

/// Visual playback-age buckets derived from sample playback history.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PlaybackAgeBucket {
    /// Samples played within the recent window, including future-skewed timestamps.
    #[default]
    Fresh,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
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
    /// Audio device and engine panel state for the native shell options view.
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
    /// Compact health state for the audio engine status chip.
    pub fn status_state(&self) -> AudioEngineChipStateModel {
        self.chip_state
    }

    /// Compact label for the audio engine status chip.
    pub fn status_label(&self) -> &str {
        &self.chip_label
    }

    /// Optional secondary audio-engine detail or error label.
    pub fn detail_label(&self) -> Option<&str> {
        self.detail_label.as_deref()
    }

    /// Primary picker group, currently mapped to output host.
    pub fn primary_group(&self) -> &AudioFieldModel {
        &self.output_host
    }

    /// Primary picker item, currently mapped to output device.
    pub fn primary_item(&self) -> &AudioFieldModel {
        &self.output_device
    }

    /// Primary picker number field, currently mapped to output sample rate.
    pub fn primary_number(&self) -> &AudioFieldModel {
        &self.output_sample_rate
    }

    /// Secondary picker group, currently mapped to input host.
    pub fn secondary_group(&self) -> &AudioFieldModel {
        &self.input_host
    }

    /// Secondary picker item, currently mapped to input device.
    pub fn secondary_item(&self) -> &AudioFieldModel {
        &self.input_device
    }

    /// Secondary picker number field, currently mapped to input sample rate.
    pub fn secondary_number(&self) -> &AudioFieldModel {
        &self.input_sample_rate
    }

    /// Currently active generic paired-picker target.
    pub fn active_picker(&self) -> Option<form::PairedPickerTarget> {
        self.active_picker.map(Into::into)
    }

    /// Option rows for the requested generic paired-picker target.
    pub fn options_for(&self, target: form::PairedPickerTarget) -> &[AudioOptionItemModel] {
        match target {
            form::PairedPickerTarget::PrimaryGroup => &self.output_host_options,
            form::PairedPickerTarget::PrimaryItem => &self.output_device_options,
            form::PairedPickerTarget::PrimaryNumber => &self.output_sample_rate_options,
            form::PairedPickerTarget::SecondaryGroup => &self.input_host_options,
            form::PairedPickerTarget::SecondaryItem => &self.input_device_options,
            form::PairedPickerTarget::SecondaryNumber => &self.input_sample_rate_options,
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
    pub fn viewport(&self) -> WaveformViewportModel {
        WaveformViewportModel::new(
            self.view_start_milli,
            self.view_end_milli,
            self.view_start_micros,
            self.view_end_micros,
            self.view_start_nanos,
            self.view_end_nanos,
        )
    }

    /// Return this panel's generic timeline transport state.
    pub fn transport(&self) -> WaveformTransportModel {
        WaveformTransportModel::new(
            self.cursor_milli,
            self.playhead_milli,
            self.playhead_micros,
            self.selection_milli,
        )
    }

    /// Return this panel's generic timeline edit preview.
    pub fn edit_preview(&self) -> WaveformEditPreviewModel {
        WaveformEditPreviewModel::new(
            self.edit_selection_milli,
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
    pub fn image_preview(&self) -> WaveformImagePreviewModel {
        WaveformImagePreviewModel::new(
            self.loaded_label.clone(),
            self.loading,
            self.image_rendering,
            self.waveform_image_signature,
            self.waveform_image.clone(),
        )
    }

    /// Return this panel's generic normalized timeline surface state.
    pub fn timeline_surface(&self) -> WaveformSurfaceModel {
        WaveformSurfaceModel::new(
            self.viewport(),
            self.transport(),
            self.edit_preview(),
            self.feedback_events(),
            self.presentation(),
            self.image_preview(),
            self.slices.clone(),
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
    pub fn signal_chrome(&self) -> WaveformChromeStateModel {
        WaveformChromeStateModel::new(
            self.transport_hint.clone(),
            self.compare_anchor_available,
            self.compare_anchor_label.clone(),
            self.channel_view,
        )
    }

    /// Return this chrome model's generic signal visualization tool state.
    pub fn signal_tools(&self) -> WaveformToolStateModel {
        WaveformToolStateModel::new(
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
        Self {
            title: String::from(crate::gui_runtime::DEFAULT_NATIVE_WINDOW_TITLE),
            backend_label: String::from("backend: native_vello"),
            sources_label: String::from("Sources"),
            status_text: String::new(),
            status: StatusBarModel {
                left: String::new(),
                center: String::from("rows: 0 | selected: 0 | anchor: - | search: -"),
                right: String::from("col: 2/3"),
            },
            audio_engine: AudioEngineModel::default(),
            browser_actions: BrowserActionsModel::default(),
            options_panel: OptionsPanelModel::default(),
            progress_overlay: ProgressOverlayModel::default(),
            confirm_prompt: ConfirmPromptModel::default(),
            drag_overlay: DragOverlayModel::default(),
            columns: [
                ColumnModel::new("Trash", 0),
                ColumnModel::new("Samples", 0),
                ColumnModel::new("Keep", 0),
            ],
            selected_column: 1,
            volume: 1.0,
            transport_running: true,
            sources: SourcesPanelModel {
                header: String::from("Sources"),
                search_query: String::new(),
                active_folder_pane: FolderPaneIdModel::Upper,
                upper_folder_pane: FolderPaneModel {
                    pane: FolderPaneIdModel::Upper,
                    title: String::from("Upper"),
                    ..FolderPaneModel::default()
                },
                lower_folder_pane: FolderPaneModel {
                    pane: FolderPaneIdModel::Lower,
                    title: String::from("Lower"),
                    ..FolderPaneModel::default()
                },
                tree_search_query: String::new(),
                show_all_items: false,
                can_toggle_show_all_items: false,
                flattened_view: false,
                can_toggle_flattened_view: false,
                selected_row: None,
                loading_row: None,
                mutation_busy_row: None,
                focused_tree_row: None,
                rows: RetainedVec::new(),
                tree_rows: RetainedVec::new(),
                tree_actions: FolderActionsModel::default(),
                recovery: FolderRecoveryModel::default(),
            },
            browser: BrowserPanelModel::default(),
            browser_chrome: BrowserChromeModel::default(),
            map: MapPanelModel::default(),
            waveform: WaveformPanelModel::default(),
            waveform_chrome: WaveformChromeModel::default(),
            update: UpdatePanelModel::default(),
            focus_context: FocusContextModel::None,
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
    fn waveform_panel_projects_generic_timeline_surface_state() {
        let model = WaveformPanelModel {
            view_start_micros: 125_000,
            playhead_micros: Some(250_250),
            selection_export_failure_flash_nonce: 5,
            loop_enabled: true,
            loaded_label: Some(String::from("Loaded")),
            slices: vec![super::WaveformSlicePreviewModel {
                range: super::NormalizedRangeModel::new(100, 200),
                selected: true,
                focused: false,
                marked_for_export: false,
                review_candidate: false,
                review_exempted: false,
            }],
            ..WaveformPanelModel::default()
        };
        let surface = model.timeline_surface();

        assert_eq!(surface.viewport.start_micros, 125_000);
        assert_eq!(surface.transport.resolved_playhead_micros(), Some(250_250));
        assert_eq!(surface.feedback_events.primary_failure_nonce, 5);
        assert!(surface.presentation.repeat_enabled);
        assert_eq!(
            surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(surface.markers.len(), 1);
    }

    #[test]
    fn native_motion_projects_generic_timeline_motion_state() {
        let model = super::NativeMotionModel {
            transport_running: true,
            map_active: false,
            active_rating_filters: [false; 8],
            active_playback_age_filters: [false; 3],
            marked_filter_active: false,
            waveform_selection_milli: Some(super::NormalizedRangeModel::new(100, 400)),
            waveform_slices: Vec::new(),
            waveform_selection_export_flash_nonce: 11,
            waveform_selection_export_failure_flash_nonce: 12,
            waveform_edit_selection_apply_flash_nonce: 13,
            waveform_edit_selection_milli: None,
            waveform_edit_fade_in_end_milli: Some(120),
            waveform_edit_fade_in_end_micros: Some(120_000),
            waveform_edit_fade_in_mute_start_milli: None,
            waveform_edit_fade_in_mute_start_micros: None,
            waveform_edit_fade_in_curve_milli: Some(200),
            waveform_edit_fade_out_start_milli: None,
            waveform_edit_fade_out_start_micros: None,
            waveform_edit_fade_out_mute_end_milli: Some(390),
            waveform_edit_fade_out_mute_end_micros: Some(390_000),
            waveform_edit_fade_out_curve_milli: Some(800),
            waveform_loop_enabled: true,
            waveform_loop_lock_enabled: true,
            waveform_cursor_milli: Some(150),
            waveform_playhead_milli: Some(250),
            waveform_playhead_micros: Some(250_500),
            waveform_view_start_milli: 10,
            waveform_view_end_milli: 900,
            waveform_view_start_micros: 10_000,
            waveform_view_end_micros: 900_000,
            waveform_view_start_nanos: 10_000_000,
            waveform_view_end_nanos: 900_000_000,
            waveform_tempo_label: Some(String::from("128 BPM")),
            waveform_zoom_label: Some(String::from("4x")),
            waveform_loaded_label: Some(String::from("Loaded")),
            waveform_loading: true,
            waveform_image_signature: Some(42),
            waveform_transport_hint: String::from("playing"),
            waveform_compare_anchor_available: true,
            waveform_compare_anchor_label: Some(String::from("A")),
            waveform_channel_view: super::WaveformChannelViewModel::Stereo,
            waveform_normalized_audition_enabled: true,
            waveform_bpm_snap_enabled: true,
            waveform_relative_bpm_grid_enabled: false,
            waveform_transient_snap_enabled: true,
            waveform_transient_markers_enabled: true,
            waveform_slice_mode_enabled: false,
            waveform_exact_duplicate_cleanup_available: true,
            status_right: String::from("ready"),
        };

        let motion = model.timeline_motion();

        assert!(motion.transport_running);
        assert_eq!(motion.surface.viewport.start_micros, 10_000);
        assert_eq!(
            motion.surface.transport.resolved_playhead_micros(),
            Some(250_500)
        );
        assert_eq!(motion.surface.feedback_events.primary_success_nonce, 11);
        assert!(motion.surface.presentation.repeat_enabled);
        assert_eq!(
            motion.surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(motion.chrome.status_hint, "playing");
        assert_eq!(
            motion.chrome.channel_view,
            super::WaveformChannelViewModel::Stereo
        );
        assert!(motion.tools.lock_enabled);
        assert!(motion.tools.cleanup_available);
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
