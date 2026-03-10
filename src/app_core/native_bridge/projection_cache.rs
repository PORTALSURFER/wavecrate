use super::metrics::{trace_projection_cache_lookup, trace_projection_segment_lookup};
use crate::app_core::actions::{NativeAppModel, NativeDirtySegments};
use crate::app_core::controller::AppController;
use crate::app_core::native_shell;
use std::sync::Arc;

/// Projection probe helpers and benchmark counters.
mod probe_metrics;
pub use probe_metrics::ProjectionRebuildCauseCounts;
/// Projection-key derivation helpers and key-partition construction.
mod projection_key;
/// Projection segment materialization helpers and retained-model update flow.
mod segment_materialize;

/// Projection segments tracked for retained model refresh and profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProjectionSegment {
    /// Footer/status string projection.
    StatusBar,
    /// Browser metadata/chrome/action projection.
    BrowserFrame,
    /// Browser visible-row window projection.
    BrowserRowsWindow,
    /// Similarity map panel projection.
    MapPanel,
    /// Waveform panel/chrome projection.
    WaveformOverlay,
}

/// Hit/miss counters for one retained projection segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentLookupCount {
    /// Number of model pulls that reused retained projection output.
    pub hit_count: u64,
    /// Number of model pulls that recomputed this projection segment.
    pub miss_count: u64,
}

/// Aggregated hit/miss counters for all retained projection segments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentLookupCounts {
    /// Status-bar segment counters.
    pub status_bar: ProjectionSegmentLookupCount,
    /// Browser-frame segment counters.
    pub browser_frame: ProjectionSegmentLookupCount,
    /// Browser rows-window segment counters.
    pub browser_rows_window: ProjectionSegmentLookupCount,
    /// Map-panel segment counters.
    pub map_panel: ProjectionSegmentLookupCount,
    /// Waveform-overlay segment counters.
    pub waveform_overlay: ProjectionSegmentLookupCount,
}

impl ProjectionSegmentLookupCounts {
    /// Record one segment-level lookup decision for the current projection pull.
    fn record_lookup(&mut self, segment: ProjectionSegment, hit: bool) {
        let counts = match segment {
            ProjectionSegment::StatusBar => &mut self.status_bar,
            ProjectionSegment::BrowserFrame => &mut self.browser_frame,
            ProjectionSegment::BrowserRowsWindow => &mut self.browser_rows_window,
            ProjectionSegment::MapPanel => &mut self.map_panel,
            ProjectionSegment::WaveformOverlay => &mut self.waveform_overlay,
        };
        if hit {
            counts.hit_count = counts.hit_count.saturating_add(1);
        } else {
            counts.miss_count = counts.miss_count.saturating_add(1);
        }
    }
}

/// Measured output from one fixed retained-projection probe loop.
///
/// The lookup counters reflect the segment reuse decisions observed during the
/// measured iterations only. `projection_p95_us` captures the measured
/// projection-stage latency of those same iterations, excluding warmup passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectionSegmentProbeMeasurement {
    /// Aggregated hit/miss counters observed during measured iterations.
    pub lookup_counts: ProjectionSegmentLookupCounts,
    /// Measured retained-projection p95 latency in microseconds.
    pub projection_p95_us: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NativeProjectionCacheKey {
    pub(super) status_revision: u64,
    pub(super) sources_selected: Option<usize>,
    pub(super) sources_len: usize,
    pub(super) folder_rows_len: usize,
    pub(super) folder_focused: Option<usize>,
    pub(super) folder_search_revision: u64,
    pub(super) browser_visible_len: usize,
    pub(super) browser_visible_rows_revision: u64,
    pub(super) browser_selected_visible: Option<usize>,
    pub(super) browser_anchor_visible: Option<usize>,
    pub(super) browser_autoscroll: bool,
    pub(super) browser_view_window_start: usize,
    pub(super) browser_render_window_start: usize,
    pub(super) browser_selected_paths_len: usize,
    pub(super) browser_selected_paths_revision: u64,
    pub(super) browser_search_revision: u64,
    pub(super) browser_filter: u8,
    pub(super) browser_sort: u8,
    pub(super) browser_tab: u8,
    pub(super) progress_visible: bool,
    pub(super) progress_completed: usize,
    pub(super) progress_total: usize,
    pub(super) prompt_active: bool,
    pub(super) drag_active: bool,
    pub(super) options_panel_visible: bool,
    pub(super) options_panel_input_monitoring_enabled: bool,
    pub(super) options_panel_advance_after_rating_enabled: bool,
    pub(super) options_panel_destructive_yolo_mode_enabled: bool,
    pub(super) options_panel_invert_waveform_scroll_enabled: bool,
    pub(super) options_panel_trash_folder_hash: Option<u64>,
    pub(super) waveform_signature: Option<u64>,
    pub(super) waveform_selection_start_milli: Option<u16>,
    pub(super) waveform_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_selection_start_milli: Option<u16>,
    pub(super) waveform_edit_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_mute_start_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_curve_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_start_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_mute_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_curve_milli: Option<u16>,
    pub(super) waveform_view_start_milli: u16,
    pub(super) waveform_view_end_milli: u16,
    pub(super) waveform_loop_enabled: bool,
    pub(super) waveform_bpm_bits: Option<u32>,
    pub(super) waveform_channel_view: u8,
    pub(super) waveform_normalized_audition_enabled: bool,
    pub(super) waveform_bpm_snap_enabled: bool,
    pub(super) waveform_transient_snap_enabled: bool,
    pub(super) waveform_transient_markers_enabled: bool,
    pub(super) waveform_slice_mode_enabled: bool,
    pub(super) map_open: bool,
    pub(super) map_zoom_bits: u32,
    pub(super) map_pan_x_bits: u32,
    pub(super) map_pan_y_bits: u32,
    pub(super) map_selection_revision: u64,
    pub(super) map_hover_revision: u64,
    pub(super) map_dataset_revision: u64,
    pub(super) map_query_revision: u64,
    pub(super) map_points_revision: u64,
    pub(super) update_status: u8,
    pub(super) update_revision: u64,
    pub(super) loaded_wav_revision: u64,
    pub(super) volume_milli: u16,
    pub(super) transport_running: bool,
    pub(super) focus_context: u8,
}

/// Status-bar projection key scoped to status and footer-affecting state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StatusProjectionCacheKey {
    status_revision: u64,
    browser_visible_len: usize,
    browser_selected_paths_len: usize,
    browser_anchor_visible: Option<usize>,
    browser_search_revision: u64,
    browser_search_busy: bool,
    inline_progress_visible: bool,
    inline_progress_completed: usize,
    inline_progress_total: usize,
    inline_progress_cancel_requested: bool,
    inline_progress_title_hash: u64,
    inline_progress_detail_hash: Option<u64>,
    selected_column: usize,
}

/// Browser metadata/chrome projection key scoped to non-row browser state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserFrameProjectionCacheKey {
    browser_visible_len: usize,
    browser_selected_visible: Option<usize>,
    browser_anchor_visible: Option<usize>,
    browser_autoscroll: bool,
    browser_view_window_start: usize,
    browser_selected_paths_len: usize,
    browser_search_revision: u64,
    browser_search_busy: bool,
    browser_sort: u8,
    browser_tab: u8,
    browser_similarity_follow_loaded: bool,
    loaded_wav_revision: u64,
}

/// Browser rows projection key scoped to windowed row content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserRowsProjectionCacheKey {
    browser_visible_rows_revision: u64,
    browser_visible_len: usize,
    browser_selected_visible: Option<usize>,
    browser_anchor_visible: Option<usize>,
    browser_autoscroll: bool,
    browser_view_window_start: usize,
    browser_render_window_start: usize,
    browser_selected_paths_len: usize,
    browser_selected_paths_revision: u64,
    browser_tab: u8,
}

/// Map-panel projection key scoped to similarity-map-affecting state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MapProjectionCacheKey {
    map_open: bool,
    map_zoom_bits: u32,
    map_pan_x_bits: u32,
    map_pan_y_bits: u32,
    map_selection_revision: u64,
    map_hover_revision: u64,
    map_dataset_revision: u64,
    map_query_revision: u64,
    map_points_revision: u64,
    browser_tab: u8,
}

/// Waveform projection key scoped to waveform panel/chrome state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WaveformProjectionCacheKey {
    pub(super) waveform_signature: Option<u64>,
    pub(super) waveform_selection_start_milli: Option<u16>,
    pub(super) waveform_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_selection_start_milli: Option<u16>,
    pub(super) waveform_edit_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_mute_start_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_curve_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_start_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_mute_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_curve_milli: Option<u16>,
    pub(super) waveform_view_start_milli: u16,
    pub(super) waveform_view_end_milli: u16,
    pub(super) waveform_loop_enabled: bool,
    pub(super) waveform_bpm_bits: Option<u32>,
    pub(super) waveform_channel_view: u8,
    pub(super) waveform_normalized_audition_enabled: bool,
    pub(super) waveform_bpm_snap_enabled: bool,
    pub(super) waveform_transient_snap_enabled: bool,
    pub(super) waveform_transient_markers_enabled: bool,
    pub(super) waveform_slice_mode_enabled: bool,
    pub(super) loaded_wav_revision: u64,
    pub(super) transport_running: bool,
}

/// Projection key for static fields that are not part of explicit segment buckets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NonSegmentStaticProjectionCacheKey {
    sources_selected: Option<usize>,
    sources_len: usize,
    folder_rows_len: usize,
    folder_focused: Option<usize>,
    folder_search_revision: u64,
    update_status: u8,
    update_revision: u64,
    volume_milli: u16,
    transport_running: bool,
    focus_context: u8,
    trash_count: usize,
    neutral_count: usize,
    keep_count: usize,
}

/// Lightweight derived projection snapshot computed before materialization.
///
/// The derive phase collects only revisions/keys and scalar selectors needed to
/// decide which materialization segments are dirty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DerivedProjectionState {
    /// Full app projection cache key for immediate whole-model hit checks.
    app_key: NativeProjectionCacheKey,
    /// Selected browser column index used by status projection.
    selected_column: usize,
    /// Status-bar segment key.
    status_key: StatusProjectionCacheKey,
    /// Browser-frame segment key.
    browser_frame_key: BrowserFrameProjectionCacheKey,
    /// Browser visible-row window segment key.
    browser_rows_key: BrowserRowsProjectionCacheKey,
    /// Similarity-map segment key.
    map_key: MapProjectionCacheKey,
    /// Waveform segment key.
    waveform_key: WaveformProjectionCacheKey,
    /// Non-segment static-field key.
    non_segment_static_key: NonSegmentStaticProjectionCacheKey,
}

impl DerivedProjectionState {
    /// Derive projection keys from controller state.
    pub(super) fn from_controller(controller: &AppController) -> Self {
        let app_key = projection_key::build_projection_cache_key(controller);
        Self::from_controller_with_app_key(controller, app_key)
    }

    /// Derive projection keys while reusing a caller-provided app key snapshot.
    pub(super) fn from_controller_with_app_key(
        controller: &AppController,
        app_key: NativeProjectionCacheKey,
    ) -> Self {
        let selected_column = native_shell::selected_column_index(&controller.ui);
        Self {
            app_key,
            selected_column,
            status_key: projection_key::build_status_projection_key(controller, selected_column),
            browser_frame_key: projection_key::build_browser_frame_projection_key(controller),
            browser_rows_key: projection_key::build_browser_rows_projection_key(controller),
            map_key: projection_key::build_map_projection_key(controller),
            waveform_key: projection_key::build_waveform_projection_key(controller),
            non_segment_static_key: projection_key::build_non_segment_static_projection_key(
                controller,
            ),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct NativeProjectionCache {
    pub(super) app_key: Option<NativeProjectionCacheKey>,
    pub(super) app_model: Option<Arc<NativeAppModel>>,
    /// Mutable retained model reused on projection misses to avoid Arc clone fallback paths.
    pub(super) app_model_working: Option<NativeAppModel>,
    pub(super) status_key: Option<StatusProjectionCacheKey>,
    pub(super) browser_frame_key: Option<BrowserFrameProjectionCacheKey>,
    pub(super) browser_rows_key: Option<BrowserRowsProjectionCacheKey>,
    pub(super) map_key: Option<MapProjectionCacheKey>,
    pub(super) waveform_key: Option<WaveformProjectionCacheKey>,
    pub(super) non_segment_static_key: Option<NonSegmentStaticProjectionCacheKey>,
    pub(super) segment_lookup_counts: ProjectionSegmentLookupCounts,
}

impl NativeProjectionCache {
    /// Record one projection segment lookup decision.
    fn record_segment_lookup(&mut self, segment: ProjectionSegment, hit: bool) {
        trace_projection_segment_lookup(segment, hit);
        self.segment_lookup_counts.record_lookup(segment, hit);
    }

    /// Return and clear segment lookup counters accumulated so far.
    pub(super) fn take_segment_lookup_counts(&mut self) -> ProjectionSegmentLookupCounts {
        std::mem::take(&mut self.segment_lookup_counts)
    }

    /// Resolve the retained app-model snapshot using derived projection state.
    pub(super) fn resolve_or_project(
        &mut self,
        controller: &mut AppController,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        segment_materialize::resolve_or_project(self, controller)
    }

    /// Resolve retained projection output using a caller-provided derive state.
    pub(super) fn resolve_or_project_with_derived(
        &mut self,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        segment_materialize::resolve_or_project_with_derived(self, controller, derived)
    }

    #[cfg(test)]
    /// Fully clear retained projection cache state.
    pub(super) fn invalidate(&mut self) {
        self.app_key = None;
        self.app_model = None;
        self.app_model_working = None;
        self.status_key = None;
        self.browser_frame_key = None;
        self.browser_rows_key = None;
        self.map_key = None;
        self.waveform_key = None;
        self.non_segment_static_key = None;
    }

    /// Invalidate only the global key so the next pull runs segment refresh.
    pub(super) fn invalidate_key_only(&mut self) {
        self.app_key = None;
    }
}

pub(super) fn build_projection_cache_key(controller: &AppController) -> NativeProjectionCacheKey {
    projection_key::build_projection_cache_key(controller)
}

/// Build a waveform projection key from the current controller snapshot.
#[cfg(test)]
pub(super) fn build_waveform_projection_key(
    controller: &AppController,
) -> WaveformProjectionCacheKey {
    projection_key::build_waveform_projection_key(controller)
}

/// Measure retained projection segment hit/miss counters over a fixed action loop.
///
/// The callback mutates controller state once per iteration. After each action
/// mutation, this helper runs native frame preparation and retained projection.
/// Warmup iterations are excluded from the returned counters.
pub fn measure_projection_segment_lookup_counts(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionSegmentLookupCounts {
    probe_metrics::measure_projection_segment_lookup_counts(
        controller,
        warmup_iters,
        measure_iters,
        &mut apply_step,
    )
}

/// Measure one retained-projection probe loop and return lookup counters plus
/// measured projection-stage latency.
///
/// The callback mutates controller state once per iteration. After each action
/// mutation, this helper runs native frame preparation and measures only the
/// retained projection step. Warmup iterations are excluded from returned
/// counters and from the reported `projection_p95_us`.
pub fn measure_projection_segment_probe(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionSegmentProbeMeasurement {
    probe_metrics::measure_projection_segment_probe(
        controller,
        warmup_iters,
        measure_iters,
        &mut apply_step,
    )
}

/// Measure rebuild-cause counters over a fixed action loop.
///
/// The callback mutates controller state once per iteration. After each action
/// mutation, this helper runs native frame preparation and retained projection.
/// When `include_motion_pull` is `true`, an additional motion-model pull runs
/// after model projection to approximate runtime motion-only refresh behavior.
/// Warmup iterations are excluded from returned counts.
pub fn measure_projection_rebuild_cause_counts(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    include_motion_pull: bool,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionRebuildCauseCounts {
    probe_metrics::measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        include_motion_pull,
        &mut apply_step,
    )
}
