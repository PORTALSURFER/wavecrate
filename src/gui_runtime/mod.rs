//! Shared GUI runtime host implementations re-exported from `radiant`.
//!
//! The runtime layer in `sempal` is intentionally minimal and has a strict
//! contract:
//!
//! * it converts `sempal` launch options into `radiant`-native options,
//! * it forwards those options to `radiant` runtime entry points,
//! * it routes runtime errors into project logging.
//!
//! No widget state, layout rules, rendering command construction, event policy,
//! diffing, or other GUI infrastructure is implemented here. Those
//! responsibilities remain in `radiant`.
//!
//! This separation allows deterministic ownership of interaction and layout logic
//! in one place while keeping host bootstrapping lightweight.
//!
//! Sempal intentionally reaches the current native shell through
//! `radiant::compat::sempal_shell` so the shell reads as compatibility
//! infrastructure rather than the preferred generic Radiant API.

use crate::app::{
    controller::ui::hotkeys::{self, KeyPress},
    state::FocusContext,
};
use crate::app_core::actions::{
    NativeAppBridge, NativeAppModel, NativeFrameBuildResult, NativeGuiAutomationSnapshot,
    NativeMotionModel, NativeUiAction,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

pub use radiant::compat::sempal_shell::NativeStartupTimingArtifact;

/// Machine-readable native shutdown timing payload exported by Sempal bridges.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NativeShutdownTimingArtifact {
    /// Whether all shutdown phases completed without a captured error.
    pub status: String,
    /// Explicit shutdown failure reason when a phase reports an error.
    pub failure_reason: Option<String>,
    /// Milliseconds spent flushing bridge-owned pending input before exit.
    pub bridge_exit_flush_ms: Option<f64>,
    /// Milliseconds spent persisting host configuration during exit.
    pub config_persist_ms: Option<f64>,
    /// Milliseconds spent draining controller job workers.
    pub controller_jobs_shutdown_ms: Option<f64>,
    /// Milliseconds spent draining analysis workers.
    pub analysis_shutdown_ms: Option<f64>,
    /// Milliseconds spent inside the controller shutdown boundary.
    pub controller_shutdown_ms: Option<f64>,
    /// Milliseconds spent inside the full runtime-exit hook.
    pub runtime_exit_total_ms: Option<f64>,
}

/// Structured runtime artifacts exported after one Sempal native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Sempal shutdown timing artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<NativeShutdownTimingArtifact>,
}

/// Result plus structured artifacts returned by one Sempal native runtime execution.
#[derive(Debug)]
pub struct NativeRunReport {
    /// Structured artifacts captured during the run.
    pub artifacts: NativeRuntimeArtifacts,
    /// Native runtime success or error outcome.
    pub result: Result<(), String>,
}

/// RGBA icon payload used by native runtime hosts.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// Native runtime launch options for Vello hosts.
#[derive(Clone, Debug, Default)]
pub struct NativeRunOptions {
    /// Window title.
    pub title: String,
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
    /// Whether the window starts maximized.
    pub maximized: bool,
    /// Whether native window decorations remain enabled.
    pub decorations: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
    /// Target frame rate for animation-driven redraws.
    pub target_fps: u32,
}

/// Converts app-level Vello launch options into the hosted `radiant` representation.
///
/// Mapping is intentionally field-for-field to preserve behavior and avoid
/// hidden launch-time mutations.
impl From<NativeRunOptions> for radiant::compat::sempal_shell::NativeRunOptions {
    fn from(value: NativeRunOptions) -> Self {
        Self {
            title: value.title,
            inner_size: value.inner_size,
            min_inner_size: value.min_inner_size,
            maximized: value.maximized,
            decorations: value.decorations,
            icon: value.icon.map(Into::into),
            target_fps: value.target_fps,
        }
    }
}

/// Converts app-level icon payloads into `radiant` host icon payloads.
///
/// All pixel bytes are forwarded unchanged; callers remain responsible for
/// supplying valid RGBA data and matching dimensions.
impl From<WindowIconRgba> for radiant::compat::sempal_shell::WindowIconRgba {
    fn from(value: WindowIconRgba) -> Self {
        Self {
            rgba: value.rgba,
            width: value.width,
            height: value.height,
        }
    }
}

struct CompatNativeAppBridge<B> {
    inner: B,
}

impl<B> CompatNativeAppBridge<B> {
    fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: NativeAppBridge> radiant::compat::sempal_shell::NativeAppBridge
    for CompatNativeAppBridge<B>
{
    fn project_model(&mut self) -> Arc<radiant::compat::sempal_shell::AppModel> {
        let model = self.inner.project_model();
        Arc::new(model.as_ref().into())
    }

    fn pull_model(&mut self) -> radiant::compat::sempal_shell::AppModel {
        self.inner.pull_model().into()
    }

    fn pull_model_arc(&mut self) -> Arc<radiant::compat::sempal_shell::AppModel> {
        let model = self.inner.pull_model_arc();
        Arc::new(model.as_ref().into())
    }

    fn project_motion_model(&mut self) -> Option<radiant::compat::sempal_shell::NativeMotionModel> {
        self.inner
            .project_motion_model()
            .map(NativeMotionModel::into)
    }

    fn take_dirty_segments(&mut self) -> radiant::compat::sempal_shell::DirtySegments {
        self.inner.take_dirty_segments().into()
    }

    fn take_segment_revisions(&mut self) -> radiant::compat::sempal_shell::SegmentRevisions {
        self.inner.take_segment_revisions().into()
    }

    fn resolve_hotkey_press(
        &mut self,
        pending_chord: Option<radiant::compat::sempal_shell::KeyPress>,
        press: radiant::compat::sempal_shell::KeyPress,
        focus: radiant::compat::sempal_shell::FocusContextModel,
    ) -> radiant::compat::sempal_shell::HotkeyResolution {
        let resolution = hotkeys::resolve_hotkey_press(
            pending_chord.map(keypress_from_radiant),
            keypress_from_radiant(press),
            focus_context_from_radiant(focus),
        );
        radiant::compat::sempal_shell::HotkeyResolution {
            action: resolution.action.map(Into::into),
            handled: resolution.handled,
            pending_chord: resolution.pending_chord.map(keypress_to_radiant),
        }
    }

    fn reduce_action(&mut self, action: radiant::compat::sempal_shell::UiAction) {
        self.inner.reduce_action(NativeUiAction::from(action));
    }

    fn take_last_action_handled(&mut self) -> Option<bool> {
        self.inner.take_last_action_handled()
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.inner.install_repaint_signal(signal);
    }

    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, hwnd: isize) {
        self.inner.set_external_drag_hwnd(hwnd);
    }

    #[cfg(target_os = "windows")]
    fn maybe_launch_external_drag(&mut self, pointer_outside: bool, pointer_left: bool) -> bool {
        self.inner
            .maybe_launch_external_drag(pointer_outside, pointer_left)
    }

    fn observe_frame_result(&mut self, result: radiant::compat::sempal_shell::FrameBuildResult) {
        self.inner
            .observe_frame_result(NativeFrameBuildResult::from(result));
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.inner
            .on_runtime_exit()
            .and_then(|artifact| serde_json::to_value(artifact).ok())
    }
}

fn native_run_report_from_radiant(
    report: radiant::compat::sempal_shell::NativeRunReport,
) -> NativeRunReport {
    NativeRunReport {
        artifacts: NativeRuntimeArtifacts {
            startup_timing: report.artifacts.startup_timing,
            shutdown_timing: report
                .artifacts
                .shutdown_timing
                .and_then(|value| serde_json::from_value(value).ok()),
        },
        result: report.result,
    }
}

fn focus_context_from_radiant(
    focus: radiant::compat::sempal_shell::FocusContextModel,
) -> FocusContext {
    match focus {
        radiant::compat::sempal_shell::FocusContextModel::None => FocusContext::None,
        radiant::compat::sempal_shell::FocusContextModel::Waveform => FocusContext::Waveform,
        radiant::compat::sempal_shell::FocusContextModel::SampleBrowser => {
            FocusContext::SampleBrowser
        }
        radiant::compat::sempal_shell::FocusContextModel::SourceFolders => {
            FocusContext::SourceFolders
        }
        radiant::compat::sempal_shell::FocusContextModel::SourcesList => FocusContext::SourcesList,
    }
}

fn keypress_from_radiant(press: radiant::compat::sempal_shell::KeyPress) -> KeyPress {
    KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

fn keypress_to_radiant(press: KeyPress) -> radiant::compat::sempal_shell::KeyPress {
    radiant::compat::sempal_shell::KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

/// Run the native Vello backend with a host-provided application bridge.
///
/// This call blocks until the native host exits and returns an error if startup
/// or runtime execution fails.
pub fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    // No additional state is touched by this adapter; all control flow and
    // execution semantics remain in `radiant`.
    info!("Launching radiant native Vello runtime");
    let result = radiant::compat::sempal_shell::run_native_vello_app(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    )
    .map_err(|err| {
        error!(%err, "radiant native Vello runtime returned error");
        err
    });

    if result.is_ok() {
        info!("Radiant native Vello runtime returned successfully");
    }

    result
}

/// Run the native Vello backend with a host-provided application bridge and
/// return the structured runtime artifacts captured during the run.
pub fn run_native_vello_app_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    info!("Launching radiant native Vello runtime");
    let report = radiant::compat::sempal_shell::run_native_vello_app_with_artifacts(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    );
    let report = native_run_report_from_radiant(report);
    if let Err(err) = &report.result {
        error!(%err, "radiant native Vello runtime returned error");
    } else {
        info!("Radiant native Vello runtime returned successfully");
    }
    report
}

/// Run the native Vello backend with a declarative host bridge.
///
/// This entrypoint is equivalent to [`run_native_vello_app`] and is provided to
/// make declarative runtime usage explicit at call sites.
pub fn run_native_vello_app_declarative<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    info!("Launching radiant native Vello runtime (declarative host)");
    let result = radiant::compat::sempal_shell::run_native_vello_app_declarative(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    )
    .map_err(|err| {
        error!(%err, "radiant native Vello runtime returned error");
        err
    });

    if result.is_ok() {
        info!("Radiant native Vello runtime returned successfully");
    }

    result
}

/// Run the native Vello backend with a declarative host bridge and return the
/// structured runtime artifacts captured during the run.
pub fn run_native_vello_app_declarative_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    info!("Launching radiant native Vello runtime (declarative host)");
    let report = radiant::compat::sempal_shell::run_native_vello_app_declarative_with_artifacts(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    );
    let report = native_run_report_from_radiant(report);
    if let Err(err) = &report.result {
        error!(%err, "radiant native Vello runtime returned error");
    } else {
        info!("Radiant native Vello runtime returned successfully");
    }
    report
}

/// Run the native Vello backend preview shell for backend smoke testing.
///
/// This is typically used to validate host integration behavior without passing a
/// full application bridge.
pub fn run_native_vello_preview(options: NativeRunOptions) -> Result<(), String> {
    info!("Launching radiant native Vello preview runtime");
    let result =
        radiant::compat::sempal_shell::run_native_vello_preview(options.into()).map_err(|err| {
            error!(%err, "radiant native Vello preview returned error");
            err
        });

    if result.is_ok() {
        info!("Radiant native Vello preview returned successfully");
    }

    result
}

/// Capture a deterministic GUI automation snapshot without launching the native host.
pub fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    let compat_model = radiant::compat::sempal_shell::AppModel::from(model);
    radiant::compat::sempal_shell::capture_gui_automation_snapshot(viewport, &compat_model).into()
}

/// Capture a deterministic compatibility native-shell visual snapshot.
#[cfg(test)]
pub(crate) fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> impl serde::Serialize {
    let compat_model = radiant::compat::sempal_shell::AppModel::from(model);
    radiant::compat::sempal_shell::capture_native_shell_shot_snapshot(name, viewport, &compat_model)
}
