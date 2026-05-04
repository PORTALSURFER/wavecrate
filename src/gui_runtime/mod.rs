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
//! Sempal intentionally confines the current native-shell compatibility calls to
//! this runtime boundary while the preferred generic Radiant runtime API
//! continues to mature.

use crate::app_core::actions::{NativeAppBridge, NativeAppModel, NativeGuiAutomationSnapshot};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

mod native_shell_runtime;
mod native_vello;

pub use radiant::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, NativeStartupTimingArtifact, RuntimeRunReport,
};

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
    let result = native_shell_runtime::run_native_vello_app(options, bridge).map_err(|err| {
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
    let report = native_shell_runtime::run_native_vello_app_with_artifacts(options, bridge);
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
    let result = native_shell_runtime::run_native_vello_app(options, bridge).map_err(|err| {
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
    let report = native_shell_runtime::run_native_vello_app_with_artifacts(options, bridge);
    if let Err(err) = &report.result {
        error!(%err, "radiant native Vello runtime returned error");
    } else {
        info!("Radiant native Vello runtime returned successfully");
    }
    report
}

/// Capture a deterministic GUI automation snapshot without launching the native host.
pub fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    native_shell_runtime::capture_gui_automation_snapshot(viewport, model)
}

/// Capture a deterministic compatibility native-shell visual snapshot.
#[cfg(test)]
pub(crate) fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> impl serde::Serialize {
    native_shell_runtime::capture_native_shell_shot_snapshot(name, viewport, model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_timing_artifact_uses_generic_radiant_runtime_export() {
        let module = include_str!("mod.rs");
        let legacy_compat_export = concat!(
            "pub use radiant::compat::",
            "legacy_shell::NativeStartupTimingArtifact;"
        );

        assert!(module.contains("pub use radiant::gui_runtime::NativeStartupTimingArtifact;"));
        assert!(
            !module.contains(legacy_compat_export),
            "startup timing is a generic Radiant runtime artifact, not a Sempal compat DTO"
        );
    }

    #[test]
    fn automation_snapshot_adapter_exposes_shell_root_from_sempal_model() {
        let model = NativeAppModel::default();
        let snapshot = capture_gui_automation_snapshot([1440.0, 810.0], &model);

        assert_eq!(snapshot.root.id.0, "shell.root");
        assert_eq!(snapshot.viewport_width, 1440);
        assert_eq!(snapshot.viewport_height, 810);
    }
}
