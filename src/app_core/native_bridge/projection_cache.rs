use super::metrics::{trace_projection_cache_lookup, trace_projection_segment_lookup};
use super::projection_key_encoding::{
    encode_browser_filter, encode_browser_sort, encode_browser_tab, encode_update_status,
    normalized_f32_to_milli, normalized_f64_to_milli,
};
use crate::app_core::actions::{NativeAppModel, NativeDirtySegments, NativeMotionModel};
use crate::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use crate::app_core::native_shell;
use std::sync::Arc;

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
    pub(super) waveform_signature: Option<u64>,
    pub(super) waveform_cursor_milli: Option<u16>,
    pub(super) waveform_playhead_milli: Option<u16>,
    pub(super) waveform_selection_start_milli: Option<u16>,
    pub(super) waveform_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_selection_start_milli: Option<u16>,
    pub(super) waveform_edit_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_start_milli: Option<u16>,
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
    selected_column: usize,
}

/// Browser metadata/chrome projection key scoped to non-row browser state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserFrameProjectionCacheKey {
    browser_visible_len: usize,
    browser_selected_visible: Option<usize>,
    browser_anchor_visible: Option<usize>,
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
    pub(super) waveform_cursor_milli: Option<u16>,
    pub(super) waveform_playhead_milli: Option<u16>,
    pub(super) waveform_selection_start_milli: Option<u16>,
    pub(super) waveform_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_selection_start_milli: Option<u16>,
    pub(super) waveform_edit_selection_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_in_end_milli: Option<u16>,
    pub(super) waveform_edit_fade_out_start_milli: Option<u16>,
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
        let app_key = build_projection_cache_key(controller);
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
            status_key: build_status_projection_key(controller, selected_column),
            browser_frame_key: build_browser_frame_projection_key(controller),
            browser_rows_key: build_browser_rows_projection_key(controller),
            map_key: build_map_projection_key(controller),
            waveform_key: build_waveform_projection_key(controller),
            non_segment_static_key: build_non_segment_static_projection_key(controller),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct NativeProjectionCache {
    pub(super) app_key: Option<NativeProjectionCacheKey>,
    pub(super) app_model: Option<Arc<NativeAppModel>>,
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

    /// Copy browser metadata fields while preserving any retained row vector.
    fn apply_browser_frame(
        model: &mut NativeAppModel,
        frame: crate::app_core::actions::NativeBrowserPanelModel,
    ) {
        model.browser.visible_count = frame.visible_count;
        model.browser.selected_visible_row = frame.selected_visible_row;
        model.browser.selected_path_count = frame.selected_path_count;
        model.browser.search_query = frame.search_query;
        model.browser.search_placeholder = frame.search_placeholder;
        model.browser.busy = frame.busy;
        model.browser.sort_label = frame.sort_label;
        model.browser.active_tab_label = frame.active_tab_label;
        model.browser.focused_sample_label = frame.focused_sample_label;
        model.browser.anchor_visible_row = frame.anchor_visible_row;
    }

    /// Refresh always-on non-segment metadata that is not covered by static keys.
    fn refresh_non_segment_always_fields(model: &mut NativeAppModel, selected_column: usize) {
        model.selected_column = selected_column;
    }

    /// Refresh static non-segment app-model fields from current controller state.
    fn refresh_non_segment_static_fields(
        model: &mut NativeAppModel,
        controller: &mut AppController,
    ) {
        model.transport_running = controller.is_playing();
        model.volume = controller.ui.volume.clamp(0.0, 1.0);
        model.sources = native_shell::project_sources_model(&controller.ui);
        model.sources_label = format!("Sources ({})", model.sources.rows.len());
        model.columns = [
            crate::app_core::actions::NativeColumnModel::new(
                "Trash",
                controller.ui.browser.trash.len(),
            ),
            crate::app_core::actions::NativeColumnModel::new(
                "Samples",
                controller.ui.browser.neutral.len(),
            ),
            crate::app_core::actions::NativeColumnModel::new(
                "Keep",
                controller.ui.browser.keep.len(),
            ),
        ];
        model.update = native_shell::project_update_model(&controller.ui);
    }

    /// Refresh transient non-segment overlays from current controller state.
    fn refresh_non_segment_overlay_fields(model: &mut NativeAppModel, controller: &AppController) {
        model.progress_overlay = native_shell::project_progress_overlay_model(&controller.ui);
        model.confirm_prompt = native_shell::project_confirm_prompt_model(&controller.ui);
        model.drag_overlay = native_shell::project_drag_overlay_model(&controller.ui);
    }

    /// Return `true` when one segment key needs rematerialization.
    ///
    /// Example:
    /// - First projection: `has_retained_model == false` => always `true`.
    /// - Retained projection with identical key: `cached_key == Some(next_key)` => `false`.
    fn segment_key_changed<T: PartialEq>(
        has_retained_model: bool,
        cached_key: &Option<T>,
        next_key: &T,
    ) -> bool {
        !has_retained_model || cached_key.as_ref() != Some(next_key)
    }

    /// Materialize status/footer fields when the status segment is dirty.
    ///
    /// Example:
    /// - If `status_key` changes, this returns `true` and the caller sets
    ///   `NativeDirtySegments::STATUS_BAR`.
    /// - On key hit, this returns `false` and records a segment cache hit.
    fn materialize_status_segment(
        &mut self,
        model: &mut NativeAppModel,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
        has_retained_model: bool,
    ) -> bool {
        let changed =
            Self::segment_key_changed(has_retained_model, &self.status_key, &derived.status_key);
        if changed {
            self.record_segment_lookup(ProjectionSegment::StatusBar, false);
            model.status = native_shell::project_status_model(controller, derived.selected_column);
            model.status_text = controller.ui.status.text.clone();
            self.status_key = Some(derived.status_key.clone());
        } else {
            self.record_segment_lookup(ProjectionSegment::StatusBar, true);
        }
        changed
    }

    /// Materialize browser frame/chrome/action fields when the frame segment is dirty.
    ///
    /// Example:
    /// - Changing browser sort/tab/focus metadata updates this segment and returns `true`,
    ///   so the caller sets `NativeDirtySegments::BROWSER_FRAME`.
    /// - On cache hit this returns `false`; browser rows may still update separately.
    fn materialize_browser_frame_segment(
        &mut self,
        model: &mut NativeAppModel,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
        has_retained_model: bool,
    ) -> bool {
        let changed = Self::segment_key_changed(
            has_retained_model,
            &self.browser_frame_key,
            &derived.browser_frame_key,
        );
        if changed {
            self.record_segment_lookup(ProjectionSegment::BrowserFrame, false);
            let frame = native_shell::project_browser_panel_frame_model(controller);
            Self::apply_browser_frame(model, frame);
            model.browser_chrome = native_shell::project_browser_chrome_model(
                &controller.ui,
                model.browser.visible_count,
            );
            model.browser_actions = native_shell::project_browser_actions_model(&controller.ui);
            self.browser_frame_key = Some(derived.browser_frame_key.clone());
        } else {
            self.record_segment_lookup(ProjectionSegment::BrowserFrame, true);
        }
        changed
    }

    /// Materialize browser visible-row window when either row or frame state is dirty.
    ///
    /// Example:
    /// - If `browser_rows_key` changes, rows rematerialize and this returns `true`.
    /// - If `browser_frame_changed == true`, rows also rematerialize even when
    ///   `browser_rows_key` is unchanged, keeping row projection consistent with frame state.
    fn materialize_browser_rows_segment(
        &mut self,
        model: &mut NativeAppModel,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
        has_retained_model: bool,
        browser_frame_changed: bool,
    ) -> bool {
        let browser_rows_changed = Self::segment_key_changed(
            has_retained_model,
            &self.browser_rows_key,
            &derived.browser_rows_key,
        );
        if browser_frame_changed || browser_rows_changed {
            self.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, false);
            let mut rows = std::mem::take(&mut model.browser.rows);
            native_shell::project_browser_rows_model_into(
                controller,
                model.browser.visible_count,
                model.browser.selected_visible_row,
                model.browser.anchor_visible_row,
                &mut rows,
            );
            model.browser.rows = rows;
            self.browser_rows_key = Some(derived.browser_rows_key.clone());
            return true;
        }
        self.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, true);
        false
    }

    /// Materialize map-panel fields when the map segment is dirty.
    ///
    /// Example:
    /// - Any `map_key` revision change (pan/zoom/query/dataset) returns `true` and the caller
    ///   sets `NativeDirtySegments::MAP_PANEL`.
    fn materialize_map_segment(
        &mut self,
        model: &mut NativeAppModel,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
        has_retained_model: bool,
    ) -> bool {
        let changed =
            Self::segment_key_changed(has_retained_model, &self.map_key, &derived.map_key);
        if changed {
            self.record_segment_lookup(ProjectionSegment::MapPanel, false);
            model.map = native_shell::project_map_model(controller);
            self.map_key = Some(derived.map_key.clone());
        } else {
            self.record_segment_lookup(ProjectionSegment::MapPanel, true);
        }
        changed
    }

    /// Materialize waveform panel/chrome fields when the waveform segment is dirty.
    ///
    /// Example:
    /// - View/cursor/selection/transport-dependent waveform key changes return `true`,
    ///   and the caller sets `NativeDirtySegments::WAVEFORM_OVERLAY`.
    fn materialize_waveform_segment(
        &mut self,
        model: &mut NativeAppModel,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
        has_retained_model: bool,
    ) -> bool {
        let changed = Self::segment_key_changed(
            has_retained_model,
            &self.waveform_key,
            &derived.waveform_key,
        );
        if changed {
            self.record_segment_lookup(ProjectionSegment::WaveformOverlay, false);
            model.waveform = native_shell::project_waveform_model(controller);
            model.waveform_chrome = native_shell::project_waveform_chrome_model(&controller.ui);
            self.waveform_key = Some(derived.waveform_key.clone());
        } else {
            self.record_segment_lookup(ProjectionSegment::WaveformOverlay, true);
        }
        changed
    }

    /// Update static non-segment cache key and report whether it changed.
    ///
    /// Example:
    /// - Changing volume/update/source-count fields flips this key and returns `true`,
    ///   which maps to `NativeDirtySegments::GLOBAL_STATIC`.
    fn update_non_segment_static_key(
        &mut self,
        derived: &DerivedProjectionState,
        has_retained_model: bool,
    ) -> bool {
        let changed = Self::segment_key_changed(
            has_retained_model,
            &self.non_segment_static_key,
            &derived.non_segment_static_key,
        );
        self.non_segment_static_key = Some(derived.non_segment_static_key.clone());
        changed
    }

    /// Resolve the retained app-model snapshot using derived projection state.
    pub(super) fn resolve_or_project(
        &mut self,
        controller: &mut AppController,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        let _ = controller.refresh_projection_revision_bus();
        let derived = DerivedProjectionState::from_controller(controller);
        self.resolve_or_project_with_derived(controller, &derived)
    }

    /// Resolve retained projection output using a caller-provided derive state.
    ///
    /// Example:
    /// - Full cache hit (`app_key` unchanged): returns retained model and `NativeDirtySegments::empty()`.
    /// - Partial segment misses: rematerializes only changed segments and returns a bitmask union of
    ///   the exact dirty segment flags touched in this pass.
    pub(super) fn resolve_or_project_with_derived(
        &mut self,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        if self.app_key.as_ref() == Some(&derived.app_key)
            && let Some(model) = self.app_model.as_ref().map(Arc::clone)
        {
            trace_projection_cache_lookup(true);
            self.record_segment_lookup(ProjectionSegment::StatusBar, true);
            self.record_segment_lookup(ProjectionSegment::BrowserFrame, true);
            self.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, true);
            self.record_segment_lookup(ProjectionSegment::MapPanel, true);
            self.record_segment_lookup(ProjectionSegment::WaveformOverlay, true);
            return (model, NativeDirtySegments::empty());
        }
        trace_projection_cache_lookup(false);
        let has_retained_model = self.app_model.is_some();
        let mut model = self
            .app_model
            .take()
            .map(Arc::unwrap_or_clone)
            .unwrap_or_default();

        let mut dirty_segments = NativeDirtySegments::empty();

        if self.materialize_status_segment(&mut model, controller, derived, has_retained_model) {
            dirty_segments.insert(NativeDirtySegments::STATUS_BAR);
        }

        let browser_frame_changed = self.materialize_browser_frame_segment(
            &mut model,
            controller,
            derived,
            has_retained_model,
        );
        if browser_frame_changed {
            dirty_segments.insert(NativeDirtySegments::BROWSER_FRAME);
        }

        if self.materialize_browser_rows_segment(
            &mut model,
            controller,
            derived,
            has_retained_model,
            browser_frame_changed,
        ) {
            dirty_segments.insert(NativeDirtySegments::BROWSER_ROWS_WINDOW);
        }

        if self.materialize_map_segment(&mut model, controller, derived, has_retained_model) {
            dirty_segments.insert(NativeDirtySegments::MAP_PANEL);
        }

        if self.materialize_waveform_segment(&mut model, controller, derived, has_retained_model) {
            dirty_segments.insert(NativeDirtySegments::WAVEFORM_OVERLAY);
        }

        let non_segment_static_changed =
            self.update_non_segment_static_key(derived, has_retained_model);
        if non_segment_static_changed {
            dirty_segments.insert(NativeDirtySegments::GLOBAL_STATIC);
            Self::refresh_non_segment_static_fields(&mut model, controller);
        }

        Self::refresh_non_segment_always_fields(&mut model, derived.selected_column);
        Self::refresh_non_segment_overlay_fields(&mut model, controller);
        self.app_key = Some(derived.app_key.clone());
        let model = Arc::new(model);
        self.app_model = Some(Arc::clone(&model));
        (model, dirty_segments)
    }

    #[cfg(test)]
    /// Fully clear retained projection cache state.
    pub(super) fn invalidate(&mut self) {
        self.app_key = None;
        self.app_model = None;
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
    let waveform_millis = derive_waveform_projection_millis(controller);
    NativeProjectionCacheKey {
        status_revision: controller.ui.projection_revisions.status,
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_revision: controller.ui.projection_revisions.folder_search,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_visible_rows_revision: controller.ui.browser.visible_rows_revision,
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_selected_paths_revision: controller.ui.browser.selected_paths_revision,
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_filter: encode_browser_filter(controller.ui.browser.filter),
        browser_sort: encode_browser_sort(controller.ui.browser.sort),
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
        progress_visible: controller.ui.progress.visible,
        progress_completed: controller.ui.progress.completed,
        progress_total: controller.ui.progress.total,
        prompt_active: controller.ui.browser.pending_action.is_some()
            || controller.ui.sources.folders.pending_action.is_some()
            || controller.ui.sources.folders.new_folder.is_some()
            || controller.ui.waveform.pending_destructive.is_some(),
        drag_active: controller.ui.drag.payload.is_some(),
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        waveform_cursor_milli: waveform_millis.cursor_milli,
        waveform_playhead_milli: waveform_millis.playhead_milli,
        waveform_selection_start_milli: waveform_millis.selection_start_milli,
        waveform_selection_end_milli: waveform_millis.selection_end_milli,
        waveform_edit_selection_start_milli: waveform_millis.edit_selection_start_milli,
        waveform_edit_selection_end_milli: waveform_millis.edit_selection_end_milli,
        waveform_edit_fade_in_end_milli: waveform_millis.edit_fade_in_end_milli,
        waveform_edit_fade_out_start_milli: waveform_millis.edit_fade_out_start_milli,
        waveform_view_start_milli: waveform_millis.view_start_milli,
        waveform_view_end_milli: waveform_millis.view_end_milli,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_bpm_bits: controller.ui.waveform.bpm_value.map(f32::to_bits),
        waveform_channel_view: encode_waveform_channel_view(controller),
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selection_revision: controller.ui.projection_revisions.map_selection,
        map_hover_revision: controller.ui.projection_revisions.map_hover,
        map_dataset_revision: controller.ui.projection_revisions.map_dataset,
        map_query_revision: controller.ui.projection_revisions.map_query,
        map_points_revision: controller.ui.map.cached_points_revision,
        update_status: encode_update_status(&controller.ui.update.status),
        update_revision: controller.ui.projection_revisions.update,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
        volume_milli: normalized_f32_to_milli(controller.ui.volume),
        transport_running: controller.is_playing(),
    }
}

/// Normalized waveform projection values converted to milli-space key fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WaveformProjectionMillis {
    /// Cursor position in normalized milli-space.
    cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-space.
    playhead_milli: Option<u16>,
    /// Selection start in normalized milli-space.
    selection_start_milli: Option<u16>,
    /// Selection end in normalized milli-space.
    selection_end_milli: Option<u16>,
    /// Edit-selection start in normalized milli-space.
    edit_selection_start_milli: Option<u16>,
    /// Edit-selection end in normalized milli-space.
    edit_selection_end_milli: Option<u16>,
    /// Edit fade-in handle end in normalized milli-space.
    edit_fade_in_end_milli: Option<u16>,
    /// Edit fade-out handle start in normalized milli-space.
    edit_fade_out_start_milli: Option<u16>,
    /// View start in normalized milli-space.
    view_start_milli: u16,
    /// View end in normalized milli-space.
    view_end_milli: u16,
}

/// Derive normalized waveform projection key fields once for cache-key builders.
fn derive_waveform_projection_millis(controller: &AppController) -> WaveformProjectionMillis {
    let cursor_milli = controller.ui.waveform.cursor.map(normalized_f32_to_milli);
    let playhead_milli =
        controller
            .ui
            .waveform
            .playhead
            .visible
            .then_some(normalized_f32_to_milli(
                controller.ui.waveform.playhead.position,
            ));
    let (selection_start_milli, selection_end_milli) = controller
        .ui
        .waveform
        .selection
        .map(|selection| {
            let start = normalized_f32_to_milli(selection.start());
            let end = normalized_f32_to_milli(selection.end());
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    let (edit_selection_start_milli, edit_selection_end_milli) = controller
        .ui
        .waveform
        .edit_selection
        .map(|selection| {
            let start = normalized_f32_to_milli(selection.start());
            let end = normalized_f32_to_milli(selection.end());
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    let (edit_fade_in_end_milli, edit_fade_out_start_milli) = controller
        .ui
        .waveform
        .edit_selection
        .map(|selection| {
            let start = selection.start();
            let end = selection.end();
            let width = selection.width();
            if width <= 0.0 {
                return (None, None);
            }
            let fade_in_end = selection.fade_in().map(|fade| {
                normalized_f32_to_milli((start + (width * fade.length)).clamp(start, end))
            });
            let fade_out_start = selection.fade_out().map(|fade| {
                normalized_f32_to_milli((end - (width * fade.length)).clamp(start, end))
            });
            (fade_in_end, fade_out_start)
        })
        .unwrap_or((None, None));
    WaveformProjectionMillis {
        cursor_milli,
        playhead_milli,
        selection_start_milli,
        selection_end_milli,
        edit_selection_start_milli,
        edit_selection_end_milli,
        edit_fade_in_end_milli,
        edit_fade_out_start_milli,
        view_start_milli: normalized_f64_to_milli(controller.ui.waveform.view.start),
        view_end_milli: normalized_f64_to_milli(controller.ui.waveform.view.end),
    }
}

/// Build a status-bar projection key from the current controller snapshot.
fn build_status_projection_key(
    controller: &AppController,
    selected_column: usize,
) -> StatusProjectionCacheKey {
    StatusProjectionCacheKey {
        status_revision: controller.ui.projection_revisions.status,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_search_busy: controller.ui.browser.search_busy,
        selected_column,
    }
}

/// Build a browser-frame projection key from the current controller snapshot.
fn build_browser_frame_projection_key(
    controller: &AppController,
) -> BrowserFrameProjectionCacheKey {
    BrowserFrameProjectionCacheKey {
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_search_busy: controller.ui.browser.search_busy,
        browser_sort: encode_browser_sort(controller.ui.browser.sort),
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
        browser_similarity_follow_loaded: controller.ui.browser.similarity_sort_follow_loaded,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
    }
}

/// Build a browser-rows projection key from the current controller snapshot.
fn build_browser_rows_projection_key(controller: &AppController) -> BrowserRowsProjectionCacheKey {
    BrowserRowsProjectionCacheKey {
        browser_visible_rows_revision: controller.ui.browser.visible_rows_revision,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_selected_paths_revision: controller.ui.browser.selected_paths_revision,
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
    }
}

/// Build a map-panel projection key from the current controller snapshot.
fn build_map_projection_key(controller: &AppController) -> MapProjectionCacheKey {
    MapProjectionCacheKey {
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selection_revision: controller.ui.projection_revisions.map_selection,
        map_hover_revision: controller.ui.projection_revisions.map_hover,
        map_dataset_revision: controller.ui.projection_revisions.map_dataset,
        map_query_revision: controller.ui.projection_revisions.map_query,
        map_points_revision: controller.ui.map.cached_points_revision,
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
    }
}

/// Build a waveform projection key from the current controller snapshot.
pub(super) fn build_waveform_projection_key(
    controller: &AppController,
) -> WaveformProjectionCacheKey {
    let waveform_millis = derive_waveform_projection_millis(controller);
    WaveformProjectionCacheKey {
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        waveform_cursor_milli: waveform_millis.cursor_milli,
        waveform_playhead_milli: waveform_millis.playhead_milli,
        waveform_selection_start_milli: waveform_millis.selection_start_milli,
        waveform_selection_end_milli: waveform_millis.selection_end_milli,
        waveform_edit_selection_start_milli: waveform_millis.edit_selection_start_milli,
        waveform_edit_selection_end_milli: waveform_millis.edit_selection_end_milli,
        waveform_edit_fade_in_end_milli: waveform_millis.edit_fade_in_end_milli,
        waveform_edit_fade_out_start_milli: waveform_millis.edit_fade_out_start_milli,
        waveform_view_start_milli: waveform_millis.view_start_milli,
        waveform_view_end_milli: waveform_millis.view_end_milli,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_bpm_bits: controller.ui.waveform.bpm_value.map(f32::to_bits),
        waveform_channel_view: encode_waveform_channel_view(controller),
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
        transport_running: controller.is_playing(),
    }
}

/// Encode waveform channel-view mode for compact projection keys.
fn encode_waveform_channel_view(controller: &AppController) -> u8 {
    match controller.ui.waveform.channel_view {
        crate::waveform::WaveformChannelView::Mono => 0,
        crate::waveform::WaveformChannelView::SplitStereo => 1,
    }
}

/// Build a projection key for static model fields outside explicit segment keys.
fn build_non_segment_static_projection_key(
    controller: &AppController,
) -> NonSegmentStaticProjectionCacheKey {
    NonSegmentStaticProjectionCacheKey {
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_revision: controller.ui.projection_revisions.folder_search,
        update_status: encode_update_status(&controller.ui.update.status),
        update_revision: controller.ui.projection_revisions.update,
        volume_milli: normalized_f32_to_milli(controller.ui.volume),
        transport_running: controller.is_playing(),
        trash_count: controller.ui.browser.trash.len(),
        neutral_count: controller.ui.browser.neutral.len(),
        keep_count: controller.ui.browser.keep.len(),
    }
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
    let mut cache = NativeProjectionCache::default();
    for step in 0..warmup_iters.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let _ = cache.resolve_or_project(controller);
    }
    let _ = cache.take_segment_lookup_counts();
    for step in 0..measure_iters.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let _ = cache.resolve_or_project(controller);
    }
    cache.take_segment_lookup_counts()
}

/// Rebuild-cause counters observed while probing retained projection updates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionRebuildCauseCounts {
    /// Explicit static invalidations observed by the probe.
    ///
    /// Controller-only probes do not execute runtime scene invalidation scopes,
    /// so this counter remains zero for benchmark-mode measurements.
    pub explicit_static_rebuild_count: u64,
    /// Static rebuilds forced by dirty-segment masks during model pulls.
    pub dirty_mask_static_rebuild_count: u64,
    /// App-model pulls that produced a new retained model snapshot.
    pub bridge_model_pull_rebuild_count: u64,
    /// Motion-model-only pulls that changed motion state without model rebuild.
    pub bridge_motion_pull_rebuild_count: u64,
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
    let mut cache = NativeProjectionCache::default();
    let mut counts = ProjectionRebuildCauseCounts::default();
    let mut previous_model: Option<Arc<NativeAppModel>> = None;
    let mut previous_motion: Option<NativeMotionModel> = None;
    run_rebuild_cause_probe_iters(
        controller,
        &mut cache,
        &mut previous_model,
        &mut previous_motion,
        warmup_iters,
        include_motion_pull,
        &mut apply_step,
        false,
        &mut counts,
    );
    run_rebuild_cause_probe_iters(
        controller,
        &mut cache,
        &mut previous_model,
        &mut previous_motion,
        measure_iters,
        include_motion_pull,
        &mut apply_step,
        true,
        &mut counts,
    );
    counts
}

#[allow(clippy::too_many_arguments)]
fn run_rebuild_cause_probe_iters(
    controller: &mut AppController,
    cache: &mut NativeProjectionCache,
    previous_model: &mut Option<Arc<NativeAppModel>>,
    previous_motion: &mut Option<NativeMotionModel>,
    iterations: usize,
    include_motion_pull: bool,
    apply_step: &mut impl FnMut(&mut AppController, usize),
    count_results: bool,
    counts: &mut ProjectionRebuildCauseCounts,
) {
    for step in 0..iterations.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let (model, dirty_segments) = cache.resolve_or_project(controller);
        let model_rebuild = previous_model
            .as_ref()
            .is_none_or(|previous| !Arc::ptr_eq(previous, &model));
        *previous_model = Some(model);

        let mut motion_rebuild = false;
        if include_motion_pull {
            controller.prepare_native_frame(true);
            let motion = controller.project_native_motion_model();
            motion_rebuild = previous_motion
                .as_ref()
                .is_some_and(|previous| previous != &motion);
            *previous_motion = Some(motion);
        }

        if !count_results {
            continue;
        }
        if model_rebuild {
            counts.bridge_model_pull_rebuild_count =
                counts.bridge_model_pull_rebuild_count.saturating_add(1);
            if dirty_segments.requires_static_rebuild() {
                counts.dirty_mask_static_rebuild_count =
                    counts.dirty_mask_static_rebuild_count.saturating_add(1);
            }
        } else if include_motion_pull && motion_rebuild {
            counts.bridge_motion_pull_rebuild_count =
                counts.bridge_motion_pull_rebuild_count.saturating_add(1);
        }
    }
}
