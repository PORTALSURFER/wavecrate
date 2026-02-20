//! Native runtime bridge implementations for migration-facing runtimes.
//!
//! This module hosts the `radiant` bridge surface so runtime entrypoints can
//! depend on `app_core` instead of legacy runtime module paths.
//!
//! Bridge profiling is opt-in and controlled by the `native-bridge-metrics`
//! cargo feature. When enabled, setting `SEMPAL_NATIVE_BRIDGE_PROFILE`
//! (`1`, `true`, `on`, or `yes`, case-insensitive) enables periodic logging
//! of bridge execution timing and renderer work counts.
//!
//! When the feature is disabled, all profiling calls are compiled out to keep
//! default builds on the hot path with zero added overhead.

use crate::{
    app_core::actions::NativeAppBridge,
    app_core::actions::NativeMotionModel,
    app_core::actions::{NativeAppModel, NativeFrameBuildResult, NativeUiAction},
    app_core::controller::{
        AppController, AppControllerNativeRuntimeExt, build_native_app_controller,
    },
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
#[cfg(feature = "native-bridge-metrics")]
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
    rc::Rc,
    time::{Duration, Instant},
};
use tracing::{error, info};

#[cfg(feature = "native-bridge-metrics")]
const BRIDGE_PROFILE_INTERVAL: u64 = 240;
#[cfg(not(feature = "native-bridge-metrics"))]
const BRIDGE_PROFILE_INTERVAL: u64 = 1;

#[cfg(feature = "native-bridge-metrics")]
const BRIDGE_PROFILE_ENV: &str = "SEMPAL_NATIVE_BRIDGE_PROFILE";

/// Interaction classes tracked by native bridge profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InteractionActionClass {
    /// Wheel-like browser row movement actions.
    Wheel,
    /// Map interaction actions flowing through the bridge.
    MapPanProxy,
    /// Waveform seek/cursor/selection/zoom actions.
    Waveform,
    /// Volume slider interaction actions.
    Volume,
}

#[cfg(feature = "native-bridge-metrics")]
static PULL_MODEL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MODEL_PREP_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MODEL_PROJECT_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MOTION_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MOTION_PREP_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static PULL_MOTION_PROJECT_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static ACTION_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static ACTION_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of wheel-class interaction actions.
static ACTION_WHEEL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated wheel-class interaction action duration in nanoseconds.
static ACTION_WHEEL_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of map-proxy-class interaction actions.
static ACTION_MAP_PROXY_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated map-proxy-class interaction action duration in nanoseconds.
static ACTION_MAP_PROXY_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of waveform-class interaction actions.
static ACTION_WAVEFORM_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated waveform-class interaction action duration in nanoseconds.
static ACTION_WAVEFORM_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Total count of volume-class interaction actions.
static ACTION_VOLUME_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Accumulated volume-class interaction action duration in nanoseconds.
static ACTION_VOLUME_DURATION_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_ANIMATION_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_PRIMITIVES_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static FRAME_RESULT_TEXT_RUNS_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
static BRIDGE_PROFILE_ENABLED: OnceLock<bool> = OnceLock::new();

#[cfg(feature = "native-bridge-metrics")]
fn parse_bridge_profile_enabled(value: &str) -> bool {
    let normalized = value.trim();
    normalized == "1"
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("on")
        || normalized.eq_ignore_ascii_case("yes")
}

#[cfg(feature = "native-bridge-metrics")]
fn bridge_profiling_enabled() -> bool {
    *BRIDGE_PROFILE_ENABLED.get_or_init(|| {
        std::env::var(BRIDGE_PROFILE_ENV)
            .ok()
            .is_some_and(|value| parse_bridge_profile_enabled(&value))
    })
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
fn bridge_profiling_enabled() -> bool {
    false
}

#[cfg(feature = "native-bridge-metrics")]
fn saturating_add_duration(counter: &AtomicU64, duration: Duration) {
    let dur_ns = duration.as_nanos().min(u64::MAX as u128) as u64;
    counter.fetch_add(dur_ns, Ordering::Relaxed);
}

#[cfg(feature = "native-bridge-metrics")]
fn ms_from_ns(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

/// Classify UI actions into focused interaction profile groups.
fn classify_action_interaction(action: &NativeUiAction) -> Option<InteractionActionClass> {
    match action {
        NativeUiAction::MoveBrowserFocus { .. } => Some(InteractionActionClass::Wheel),
        NativeUiAction::SetBrowserTab { map: true } | NativeUiAction::FocusMapSample { .. } => {
            Some(InteractionActionClass::MapPanProxy)
        }
        NativeUiAction::SeekWaveform { .. }
        | NativeUiAction::SetWaveformCursor { .. }
        | NativeUiAction::SetWaveformSelectionRange { .. }
        | NativeUiAction::ClearWaveformSelection
        | NativeUiAction::ZoomWaveform { .. }
        | NativeUiAction::ZoomWaveformToSelection
        | NativeUiAction::ZoomWaveformFull => Some(InteractionActionClass::Waveform),
        NativeUiAction::SetVolume { .. } | NativeUiAction::CommitVolumeSetting => {
            Some(InteractionActionClass::Volume)
        }
        _ => None,
    }
}

#[cfg(feature = "native-bridge-metrics")]
fn maybe_log_bridge_profile() {
    let pull_model_count = PULL_MODEL_COUNT.load(Ordering::Relaxed);
    let pull_model_prep = PULL_MODEL_PREP_NS.load(Ordering::Relaxed);
    let pull_model_project = PULL_MODEL_PROJECT_NS.load(Ordering::Relaxed);
    let pull_motion_count = PULL_MOTION_COUNT.load(Ordering::Relaxed);
    let pull_motion_prep = PULL_MOTION_PREP_NS.load(Ordering::Relaxed);
    let pull_motion_project = PULL_MOTION_PROJECT_NS.load(Ordering::Relaxed);
    let action_count = ACTION_COUNT.load(Ordering::Relaxed);
    let action_ns = ACTION_DURATION_NS.load(Ordering::Relaxed);
    let wheel_count = ACTION_WHEEL_COUNT.load(Ordering::Relaxed);
    let wheel_ns = ACTION_WHEEL_DURATION_NS.load(Ordering::Relaxed);
    let map_proxy_count = ACTION_MAP_PROXY_COUNT.load(Ordering::Relaxed);
    let map_proxy_ns = ACTION_MAP_PROXY_DURATION_NS.load(Ordering::Relaxed);
    let waveform_count = ACTION_WAVEFORM_COUNT.load(Ordering::Relaxed);
    let waveform_ns = ACTION_WAVEFORM_DURATION_NS.load(Ordering::Relaxed);
    let volume_count = ACTION_VOLUME_COUNT.load(Ordering::Relaxed);
    let volume_ns = ACTION_VOLUME_DURATION_NS.load(Ordering::Relaxed);
    let frame_count = FRAME_RESULT_COUNT.load(Ordering::Relaxed);
    let frame_anim_count = FRAME_RESULT_ANIMATION_COUNT.load(Ordering::Relaxed);
    let primitive_sum = FRAME_RESULT_PRIMITIVES_TOTAL.load(Ordering::Relaxed);
    let text_run_sum = FRAME_RESULT_TEXT_RUNS_TOTAL.load(Ordering::Relaxed);
    let pull_model_avg_prep_ms = if pull_model_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_model_prep) / pull_model_count as f64
    };
    let pull_model_avg_project_ms = if pull_model_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_model_project) / pull_model_count as f64
    };
    let pull_motion_avg_prep_ms = if pull_motion_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_motion_prep) / pull_motion_count as f64
    };
    let pull_motion_avg_project_ms = if pull_motion_count == 0 {
        0.0
    } else {
        ms_from_ns(pull_motion_project) / pull_motion_count as f64
    };
    let action_avg_ms = if action_count == 0 {
        0.0
    } else {
        ms_from_ns(action_ns) / action_count as f64
    };
    let wheel_avg_ms = if wheel_count == 0 {
        0.0
    } else {
        ms_from_ns(wheel_ns) / wheel_count as f64
    };
    let map_proxy_avg_ms = if map_proxy_count == 0 {
        0.0
    } else {
        ms_from_ns(map_proxy_ns) / map_proxy_count as f64
    };
    let waveform_avg_ms = if waveform_count == 0 {
        0.0
    } else {
        ms_from_ns(waveform_ns) / waveform_count as f64
    };
    let volume_avg_ms = if volume_count == 0 {
        0.0
    } else {
        ms_from_ns(volume_ns) / volume_count as f64
    };
    let avg_primitives_per_frame = if frame_count == 0 {
        0.0
    } else {
        primitive_sum as f64 / frame_count as f64
    };
    let avg_text_runs_per_frame = if frame_count == 0 {
        0.0
    } else {
        text_run_sum as f64 / frame_count as f64
    };
    info!(
        pull_model_count,
        pull_motion_count,
        action_count,
        wheel_count,
        map_proxy_count,
        waveform_count,
        volume_count,
        frame_count,
        frame_anim_count,
        "native bridge profiling: pull_model prep_ms={:.3} project_ms={:.3} \
         pull_motion prep_ms={:.3} project_ms={:.3} action_ms={:.3} \
         wheel_action_ms={:.3} map_proxy_action_ms={:.3} waveform_action_ms={:.3} volume_action_ms={:.3} \
         avg_primitives_per_frame={:.2} avg_text_runs_per_frame={:.2}",
        pull_model_avg_prep_ms,
        pull_model_avg_project_ms,
        pull_motion_avg_prep_ms,
        pull_motion_avg_project_ms,
        action_avg_ms,
        wheel_avg_ms,
        map_proxy_avg_ms,
        waveform_avg_ms,
        volume_avg_ms,
        avg_primitives_per_frame,
        avg_text_runs_per_frame
    );
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline]
fn maybe_log_bridge_profile() {}

#[cfg(feature = "native-bridge-metrics")]
fn trace_pull_model_call() -> u64 {
    PULL_MODEL_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_model_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
fn trace_pull_motion_call() -> u64 {
    PULL_MOTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_motion_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
fn trace_action_call() -> u64 {
    ACTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_action_call() -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
fn trace_frame_result(result: &NativeFrameBuildResult) -> u64 {
    let frame_count = FRAME_RESULT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if result.needs_animation {
        FRAME_RESULT_ANIMATION_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    FRAME_RESULT_PRIMITIVES_TOTAL.fetch_add(result.primitive_count as u64, Ordering::Relaxed);
    FRAME_RESULT_TEXT_RUNS_TOTAL.fetch_add(result.text_run_count as u64, Ordering::Relaxed);
    frame_count
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_frame_result(_result: &NativeFrameBuildResult) -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_model_preparation(duration: Duration) {
    saturating_add_duration(&PULL_MODEL_PREP_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_model_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_model_projection(duration: Duration) {
    saturating_add_duration(&PULL_MODEL_PROJECT_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_model_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_motion_preparation(duration: Duration) {
    saturating_add_duration(&PULL_MOTION_PREP_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_motion_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_pull_motion_projection(duration: Duration) {
    saturating_add_duration(&PULL_MOTION_PROJECT_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_pull_motion_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
fn trace_action_duration(duration: Duration) {
    saturating_add_duration(&ACTION_DURATION_NS, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_action_duration(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track classified interaction action timings for bridge profiling logs.
fn trace_action_interaction(kind: InteractionActionClass, duration: Duration) {
    match kind {
        InteractionActionClass::Wheel => {
            ACTION_WHEEL_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_WHEEL_DURATION_NS, duration);
        }
        InteractionActionClass::MapPanProxy => {
            ACTION_MAP_PROXY_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_MAP_PROXY_DURATION_NS, duration);
        }
        InteractionActionClass::Waveform => {
            ACTION_WAVEFORM_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_WAVEFORM_DURATION_NS, duration);
        }
        InteractionActionClass::Volume => {
            ACTION_VOLUME_COUNT.fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&ACTION_VOLUME_DURATION_NS, duration);
        }
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op classified interaction recorder for non-profiling builds.
fn trace_action_interaction(_kind: InteractionActionClass, _duration: Duration) {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MapQueryBoundsKey {
    min_x_bits: u32,
    max_x_bits: u32,
    min_y_bits: u32,
    max_y_bits: u32,
}

/// Hash an arbitrary projection field into a compact cache-key scalar.
fn hash_projection_field<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Hash an optional string field into an optional cache-key scalar.
fn hash_optional_string(value: Option<&str>) -> Option<u64> {
    value.map(hash_projection_field)
}

impl MapQueryBoundsKey {
    fn from_bounds(bounds: crate::app_core::state::MapQueryBounds) -> Self {
        Self {
            min_x_bits: bounds.min_x.to_bits(),
            max_x_bits: bounds.max_x.to_bits(),
            min_y_bits: bounds.min_y.to_bits(),
            max_y_bits: bounds.max_y.to_bits(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeProjectionCacheKey {
    status_text_hash: u64,
    status_tone: u8,
    sources_selected: Option<usize>,
    sources_len: usize,
    folder_rows_len: usize,
    folder_focused: Option<usize>,
    folder_search_query_hash: u64,
    browser_visible_len: usize,
    browser_selected_visible: Option<usize>,
    browser_anchor_visible: Option<usize>,
    browser_selected_paths_len: usize,
    browser_search_query_hash: u64,
    browser_filter: u8,
    browser_sort: u8,
    browser_tab: u8,
    progress_visible: bool,
    progress_completed: usize,
    progress_total: usize,
    prompt_active: bool,
    drag_active: bool,
    waveform_signature: Option<u64>,
    map_open: bool,
    map_zoom_bits: u32,
    map_pan_x_bits: u32,
    map_pan_y_bits: u32,
    map_selected_sample_id_hash: Option<u64>,
    map_hovered_sample_id_hash: Option<u64>,
    map_umap_version_hash: u64,
    map_bounds_source_id_hash: Option<u64>,
    map_bounds_umap_version_hash: Option<u64>,
    map_points_source_id_hash: Option<u64>,
    map_points_umap_version_hash: Option<u64>,
    map_last_query: Option<MapQueryBoundsKey>,
    map_points_revision: u64,
    update_status: u8,
    update_available_tag_hash: Option<u64>,
    update_available_url_hash: Option<u64>,
    update_last_error_hash: Option<u64>,
    loaded_wav_hash: Option<u64>,
    volume_milli: u16,
    transport_running: bool,
}

#[derive(Clone, Debug, Default)]
struct NativeProjectionCache {
    app_key: Option<NativeProjectionCacheKey>,
    app_model: Option<NativeAppModel>,
}

impl NativeProjectionCache {
    fn resolve_or_project(
        &mut self,
        controller: &mut AppController,
        project: impl FnOnce(&mut AppController) -> NativeAppModel,
    ) -> NativeAppModel {
        let key = build_projection_cache_key(controller);
        if self.app_key.as_ref() == Some(&key)
            && let Some(model) = self.app_model.as_ref()
        {
            return model.clone();
        }
        let model = project(controller);
        self.app_key = Some(key);
        self.app_model = Some(model.clone());
        model
    }

    fn invalidate(&mut self) {
        self.app_key = None;
        self.app_model = None;
    }
}

fn build_projection_cache_key(controller: &AppController) -> NativeProjectionCacheKey {
    use crate::app_core::state::{
        MapQueryBounds, SampleBrowserSort, SampleBrowserTab, StatusTone, TriageFlagFilter,
        UpdateStatus,
    };
    let map_last_query = controller
        .ui
        .map
        .last_query
        .map(|bounds: MapQueryBounds| MapQueryBoundsKey::from_bounds(bounds));
    NativeProjectionCacheKey {
        status_text_hash: hash_projection_field(&controller.ui.status.text),
        status_tone: match controller.ui.status.status_tone {
            StatusTone::Idle => 0,
            StatusTone::Busy => 1,
            StatusTone::Info => 2,
            StatusTone::Warning => 3,
            StatusTone::Error => 4,
        },
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_query_hash: hash_projection_field(
            &controller.ui.sources.folders.search_query,
        ),
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_search_query_hash: hash_projection_field(&controller.ui.browser.search_query),
        browser_filter: match controller.ui.browser.filter {
            TriageFlagFilter::All => 0,
            TriageFlagFilter::Keep => 1,
            TriageFlagFilter::Trash => 2,
            TriageFlagFilter::Untagged => 3,
        },
        browser_sort: match controller.ui.browser.sort {
            SampleBrowserSort::ListOrder => 0,
            SampleBrowserSort::Similarity => 1,
            SampleBrowserSort::PlaybackAgeAsc => 2,
            SampleBrowserSort::PlaybackAgeDesc => 3,
        },
        browser_tab: match controller.ui.browser.active_tab {
            SampleBrowserTab::List => 0,
            SampleBrowserTab::Map => 1,
        },
        progress_visible: controller.ui.progress.visible,
        progress_completed: controller.ui.progress.completed,
        progress_total: controller.ui.progress.total,
        prompt_active: controller.ui.browser.pending_action.is_some()
            || controller.ui.sources.folders.pending_action.is_some()
            || controller.ui.sources.folders.new_folder.is_some()
            || controller.ui.waveform.pending_destructive.is_some(),
        drag_active: controller.ui.drag.payload.is_some(),
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selected_sample_id_hash: hash_optional_string(
            controller.ui.map.selected_sample_id.as_deref(),
        ),
        map_hovered_sample_id_hash: hash_optional_string(
            controller.ui.map.hovered_sample_id.as_deref(),
        ),
        map_umap_version_hash: hash_projection_field(&controller.ui.map.umap_version),
        map_bounds_source_id_hash: hash_optional_string(
            controller.ui.map.cached_bounds_source_id.as_deref(),
        ),
        map_bounds_umap_version_hash: hash_optional_string(
            controller.ui.map.cached_bounds_umap_version.as_deref(),
        ),
        map_points_source_id_hash: hash_optional_string(
            controller.ui.map.cached_points_source_id.as_deref(),
        ),
        map_points_umap_version_hash: hash_optional_string(
            controller.ui.map.cached_points_umap_version.as_deref(),
        ),
        map_last_query,
        map_points_revision: controller.ui.map.cached_points_revision,
        update_status: match controller.ui.update.status {
            UpdateStatus::Idle => 0,
            UpdateStatus::Checking => 1,
            UpdateStatus::UpdateAvailable => 2,
            UpdateStatus::Error => 3,
        },
        update_available_tag_hash: hash_optional_string(
            controller.ui.update.available_tag.as_deref(),
        ),
        update_available_url_hash: hash_optional_string(
            controller.ui.update.available_url.as_deref(),
        ),
        update_last_error_hash: hash_optional_string(controller.ui.update.last_error.as_deref()),
        loaded_wav_hash: controller
            .ui
            .loaded_wav
            .as_ref()
            .map(|path| hash_projection_field(path.as_os_str())),
        volume_milli: (controller.ui.volume.clamp(0.0, 1.0) * 1000.0).round() as u16,
        transport_running: controller.is_playing(),
    }
}

/// Return whether an action requires unconditional projection-cache invalidation.
fn action_requires_projection_cache_invalidation(action: &NativeUiAction) -> bool {
    !matches!(
        action,
        NativeUiAction::SeekWaveform { .. }
            | NativeUiAction::SetWaveformCursor { .. }
            | NativeUiAction::SetWaveformSelectionRange { .. }
            | NativeUiAction::ClearWaveformSelection
            | NativeUiAction::ZoomWaveform { .. }
            | NativeUiAction::ZoomWaveformToSelection
            | NativeUiAction::ZoomWaveformFull
            | NativeUiAction::SetVolume { .. }
            | NativeUiAction::CommitVolumeSetting
    )
}

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: AppController,
    projection_cache: NativeProjectionCache,
    /// Coalesced pending browser-focus delta from high-frequency wheel/arrow actions.
    pending_browser_focus_delta: i16,
}

impl SempalNativeBridge {
    /// Build a new native bridge initialized with persisted sempal configuration.
    pub fn new(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
    ) -> Result<Self, String> {
        info!("Building native bridge controller");
        let controller = build_native_app_controller(renderer, player).map_err(|err| {
            error!(err = %err, "Failed to build native app controller");
            err
        })?;
        info!("Native bridge controller ready");
        Ok(Self {
            controller,
            projection_cache: NativeProjectionCache::default(),
            pending_browser_focus_delta: 0,
        })
    }

    /// Queue browser-focus movement so repeated wheel/arrow actions can coalesce.
    fn enqueue_browser_focus_delta(&mut self, delta: i8) {
        self.pending_browser_focus_delta = self
            .pending_browser_focus_delta
            .saturating_add(i16::from(delta))
            .clamp(i16::from(i8::MIN), i16::from(i8::MAX));
    }

    /// Apply any queued browser-focus movement before pulling/projecting state.
    fn flush_pending_browser_focus_delta(&mut self) {
        let pending = self.pending_browser_focus_delta;
        if pending == 0 {
            return;
        }
        self.pending_browser_focus_delta = 0;
        self.projection_cache.invalidate();
        self.controller.focus_browser_delta_action(pending as i8);
    }
}

impl NativeAppBridge for SempalNativeBridge {
    fn pull_model(&mut self) -> NativeAppModel {
        let call = trace_pull_model_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        if call <= 24 {
            info!(call, "native bridge: pull_model start");
        }
        self.flush_pending_browser_focus_delta();
        self.controller.prepare_native_frame(false);
        let prepare_duration = prepare_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_model_preparation(prepare_duration);
        }
        let project_start = profiling.then(Instant::now);
        let model = self
            .projection_cache
            .resolve_or_project(&mut self.controller, |controller| {
                controller.project_native_app_model()
            });
        let project_duration = project_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_model_projection(project_duration);
        }
        if call <= 24 {
            info!(
                call,
                transport_running = model.transport_running,
                browser_visible = model.browser.visible_count,
                status_len = model.status_text.len(),
                "native bridge: pull_model completed"
            );
        }
        if profiling && call.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
        model
    }

    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        let call = trace_pull_motion_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model start");
        }
        self.flush_pending_browser_focus_delta();
        self.controller.prepare_native_frame(true);
        let prepare_duration = prepare_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_motion_preparation(prepare_duration);
        }
        let project_start = profiling.then(Instant::now);
        let model = Some(self.controller.project_native_motion_model());
        let project_duration = project_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_motion_projection(project_duration);
        }
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model completed");
        }
        if profiling && call.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
        model
    }

    fn on_action(&mut self, action: NativeUiAction) {
        if let NativeUiAction::MoveBrowserFocus { delta } = action {
            let call = trace_action_call();
            let profiling = bridge_profiling_enabled();
            let action_start = profiling.then(Instant::now);
            if call <= 64 {
                info!(call, delta, "native bridge: queue MoveBrowserFocus");
            }
            self.enqueue_browser_focus_delta(delta);
            if profiling {
                let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_action_duration(action_duration);
                trace_action_interaction(InteractionActionClass::Wheel, action_duration);
            }
            return;
        }
        self.flush_pending_browser_focus_delta();
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let interaction_class = classify_action_interaction(&action);
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            info!(call, action = ?action, "native bridge: on_action");
        }
        if action_requires_projection_cache_invalidation(&action) {
            self.projection_cache.invalidate();
        }
        self.controller.apply_native_ui_action(action);
        if profiling {
            let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_action_duration(action_duration);
            if let Some(kind) = interaction_class {
                trace_action_interaction(kind, action_duration);
            }
        }
    }

    fn on_frame_result(&mut self, result: NativeFrameBuildResult) {
        let profiling = bridge_profiling_enabled();
        if !profiling {
            return;
        }
        let frame_count = trace_frame_result(&result);
        if frame_count.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
    }

    fn on_exit(&mut self) {
        self.flush_pending_browser_focus_delta();
        if let Err(err) = self.controller.persist_native_exit_config() {
            error!(err = %err, "Failed to persist config on native exit");
            eprintln!("{err}");
            return;
        }
        info!("Persisted config on native exit");
    }
}

/// Construct a native runtime bridge for the current `sempal` controller stack.
///
/// This is the application-facing constructor used by host integrations. It
/// keeps bridge construction in `app_core` and returns a controller-backed
/// implementation of `NativeAppBridge` that delegates all GUI work to `radiant`.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<SempalNativeBridge, String> {
    SempalNativeBridge::new(renderer, player)
}

#[cfg(test)]
mod tests {
    use super::{NativeProjectionCache, SempalNativeBridge, build_projection_cache_key};
    use crate::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
    use crate::app_core::state::UpdateStatus;
    use crate::waveform::WaveformRenderer;

    #[test]
    fn projection_cache_key_changes_when_map_cache_revision_changes() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let first = build_projection_cache_key(&controller);
        controller.ui.map.cached_points_revision += 1;
        let second = build_projection_cache_key(&controller);
        assert_ne!(first, second);
    }

    #[test]
    fn projection_cache_key_changes_when_update_status_changes() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let first = build_projection_cache_key(&controller);
        controller.ui.update.status = UpdateStatus::Checking;
        let second = build_projection_cache_key(&controller);
        assert_ne!(first, second);
    }

    #[test]
    fn projection_cache_reuses_model_when_key_unchanged() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let mut cache = NativeProjectionCache::default();
        let mut projections = 0usize;

        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        assert_eq!(projections, 1);

        controller.ui.status.text = String::from("changed");
        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        assert_eq!(projections, 2);
    }

    #[test]
    fn projection_cache_invalidate_forces_refresh() {
        let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
        let mut cache = NativeProjectionCache::default();
        let mut projections = 0usize;

        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });
        cache.invalidate();
        let _ = cache.resolve_or_project(&mut controller, |controller| {
            projections += 1;
            controller.project_native_app_model()
        });

        assert_eq!(projections, 2);
    }

    /// Queued browser focus deltas should clamp into i8-safe bounds.
    #[test]
    fn browser_focus_delta_queue_coalesces_and_clamps() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: NativeProjectionCache::default(),
            pending_browser_focus_delta: 0,
        };

        bridge.enqueue_browser_focus_delta(70);
        bridge.enqueue_browser_focus_delta(70);
        assert_eq!(bridge.pending_browser_focus_delta, i16::from(i8::MAX));

        bridge.enqueue_browser_focus_delta(-120);
        assert_eq!(bridge.pending_browser_focus_delta, 7);
    }

    /// Flushing queued focus movement should invalidate projection cache keys.
    #[test]
    fn flush_pending_browser_focus_clears_projection_cache_key() {
        let controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let cache = NativeProjectionCache {
            app_key: Some(build_projection_cache_key(&controller)),
            ..NativeProjectionCache::default()
        };

        let mut bridge = SempalNativeBridge {
            controller,
            projection_cache: cache,
            pending_browser_focus_delta: 0,
        };
        bridge.enqueue_browser_focus_delta(1);
        bridge.flush_pending_browser_focus_delta();

        assert_eq!(bridge.pending_browser_focus_delta, 0);
        assert!(bridge.projection_cache.app_key.is_none());
    }

    #[cfg(feature = "native-bridge-metrics")]
    #[test]
    fn parse_bridge_profile_enabled_is_case_insensitive() {
        assert!(super::parse_bridge_profile_enabled("TRUE"));
        assert!(super::parse_bridge_profile_enabled("on"));
        assert!(super::parse_bridge_profile_enabled("Yes"));
        assert!(super::parse_bridge_profile_enabled("  true  "));
        assert!(!super::parse_bridge_profile_enabled("0"));
        assert!(!super::parse_bridge_profile_enabled("no"));
        assert!(!super::parse_bridge_profile_enabled(""));
    }
}
