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
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
#[cfg(feature = "native-bridge-metrics")]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};
use tracing::{error, info};

#[cfg(feature = "native-bridge-metrics")]
const BRIDGE_PROFILE_INTERVAL: u64 = 240;
#[cfg(not(feature = "native-bridge-metrics"))]
const BRIDGE_PROFILE_INTERVAL: u64 = 1;

#[cfg(feature = "native-bridge-metrics")]
const BRIDGE_PROFILE_ENV: &str = "SEMPAL_NATIVE_BRIDGE_PROFILE";

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
        frame_count,
        frame_anim_count,
        "native bridge profiling: pull_model prep_ms={:.3} project_ms={:.3} \
         pull_motion prep_ms={:.3} project_ms={:.3} action_ms={:.3} \
         avg_primitives_per_frame={:.2} avg_text_runs_per_frame={:.2}",
        pull_model_avg_prep_ms,
        pull_model_avg_project_ms,
        pull_motion_avg_prep_ms,
        pull_motion_avg_project_ms,
        action_avg_ms,
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

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: AppController,
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
        Ok(Self { controller })
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
        self.controller.prepare_native_frame(false);
        let prepare_duration = prepare_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_model_preparation(prepare_duration);
        }
        let project_start = profiling.then(Instant::now);
        let model = self.controller.project_native_app_model();
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
        if profiling && call % BRIDGE_PROFILE_INTERVAL == 0 {
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
        if profiling && call % BRIDGE_PROFILE_INTERVAL == 0 {
            maybe_log_bridge_profile();
        }
        model
    }

    fn on_action(&mut self, action: NativeUiAction) {
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            info!(call, action = ?action, "native bridge: on_action");
        }
        self.controller.apply_native_ui_action(action);
        if profiling {
            let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_action_duration(action_duration);
        }
    }

    fn on_frame_result(&mut self, result: NativeFrameBuildResult) {
        let profiling = bridge_profiling_enabled();
        if !profiling {
            return;
        }
        let frame_count = trace_frame_result(&result);
        if frame_count % BRIDGE_PROFILE_INTERVAL == 0 {
            maybe_log_bridge_profile();
        }
    }

    fn on_exit(&mut self) {
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
#[cfg(feature = "native-bridge-metrics")]
mod tests {
    use super::parse_bridge_profile_enabled;

    #[test]
    fn parse_bridge_profile_enabled_is_case_insensitive() {
        assert!(parse_bridge_profile_enabled("TRUE"));
        assert!(parse_bridge_profile_enabled("on"));
        assert!(parse_bridge_profile_enabled("Yes"));
        assert!(parse_bridge_profile_enabled("  true  "));
        assert!(!parse_bridge_profile_enabled("0"));
        assert!(!parse_bridge_profile_enabled("no"));
        assert!(!parse_bridge_profile_enabled(""));
    }
}
