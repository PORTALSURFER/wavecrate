//! Retained projection cache state and derived-key snapshots.

use crate::app_core::actions::{NativeAppModel, NativeDirtySegments};
use crate::app_core::controller::AppController;
use crate::app_core::native_shell;
use std::sync::Arc;

use super::projection_key;
use super::trace_projection_segment_lookup;
use super::{
    BrowserFrameProjectionCacheKey, BrowserRowsProjectionCacheKey, MapProjectionCacheKey,
    NativeProjectionCacheKey, NonSegmentStaticProjectionCacheKey, ProjectionSegment,
    ProjectionSegmentLookupCounts, StatusProjectionCacheKey, WaveformProjectionCacheKey,
    segment_materialize,
};

/// Lightweight derived projection snapshot computed before materialization.
///
/// The derive phase collects only revisions/keys and scalar selectors needed to
/// decide which materialization segments are dirty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DerivedProjectionState {
    /// Full app projection cache key for immediate whole-model hit checks.
    pub(crate) app_key: NativeProjectionCacheKey,
    /// Selected browser column index used by status projection.
    pub(crate) selected_column: usize,
    /// Status-bar segment key.
    pub(crate) status_key: StatusProjectionCacheKey,
    /// Browser-frame segment key.
    pub(crate) browser_frame_key: BrowserFrameProjectionCacheKey,
    /// Browser visible-row window segment key.
    pub(crate) browser_rows_key: BrowserRowsProjectionCacheKey,
    /// Similarity-map segment key.
    pub(crate) map_key: MapProjectionCacheKey,
    /// Waveform segment key.
    pub(crate) waveform_key: WaveformProjectionCacheKey,
    /// Non-segment static-field key.
    pub(crate) non_segment_static_key: NonSegmentStaticProjectionCacheKey,
}

impl DerivedProjectionState {
    /// Derive projection keys from controller state.
    pub(crate) fn from_controller(controller: &AppController) -> Self {
        let app_key = projection_key::build_projection_cache_key(controller);
        Self::from_controller_with_app_key(controller, app_key)
    }

    /// Derive projection keys while reusing a caller-provided app key snapshot.
    pub(crate) fn from_controller_with_app_key(
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

/// Retained app-model cache and segment keys used for projection reuse.
#[derive(Clone, Debug, Default)]
pub(crate) struct NativeProjectionCache {
    pub(crate) app_key: Option<NativeProjectionCacheKey>,
    pub(crate) app_model: Option<Arc<NativeAppModel>>,
    pub(crate) status_key: Option<StatusProjectionCacheKey>,
    pub(crate) browser_frame_key: Option<BrowserFrameProjectionCacheKey>,
    pub(crate) browser_rows_key: Option<BrowserRowsProjectionCacheKey>,
    pub(crate) map_key: Option<MapProjectionCacheKey>,
    pub(crate) waveform_key: Option<WaveformProjectionCacheKey>,
    pub(crate) non_segment_static_key: Option<NonSegmentStaticProjectionCacheKey>,
    pub(crate) segment_lookup_counts: ProjectionSegmentLookupCounts,
}

impl NativeProjectionCache {
    /// Record one projection segment lookup decision.
    pub(crate) fn record_segment_lookup(&mut self, segment: ProjectionSegment, hit: bool) {
        trace_projection_segment_lookup(segment, hit);
        self.segment_lookup_counts.record_lookup(segment, hit);
    }

    /// Return and clear segment lookup counters accumulated so far.
    pub(crate) fn take_segment_lookup_counts(&mut self) -> ProjectionSegmentLookupCounts {
        std::mem::take(&mut self.segment_lookup_counts)
    }

    /// Resolve the retained app-model snapshot using derived projection state.
    pub(crate) fn resolve_or_project(
        &mut self,
        controller: &mut AppController,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        segment_materialize::resolve_or_project(self, controller)
    }

    /// Resolve retained projection output using a caller-provided derive state.
    pub(crate) fn resolve_or_project_with_derived(
        &mut self,
        controller: &mut AppController,
        derived: &DerivedProjectionState,
    ) -> (Arc<NativeAppModel>, NativeDirtySegments) {
        segment_materialize::resolve_or_project_with_derived(self, controller, derived)
    }

    #[cfg(test)]
    /// Fully clear retained projection cache state.
    pub(crate) fn invalidate(&mut self) {
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
    pub(crate) fn invalidate_key_only(&mut self) {
        self.app_key = None;
    }
}
