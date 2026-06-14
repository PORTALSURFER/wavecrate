use super::NativeStartupTimingArtifact;
use serde::{Deserialize, Serialize};

/// Machine-readable native shutdown timing payload exported by Wavecrate bridges.
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

/// Structured runtime artifacts exported after one Wavecrate native run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<NativeStartupTimingArtifact>,
    /// Wavecrate shutdown timing artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<NativeShutdownTimingArtifact>,
}

/// Result plus structured artifacts returned by one Wavecrate UI runtime execution.
#[derive(Debug)]
pub struct NativeRunReport {
    /// Structured artifacts captured during the run.
    pub artifacts: NativeRuntimeArtifacts,
    /// UI runtime success or error outcome.
    pub result: Result<(), String>,
}
