/// Structured runtime artifacts exported after one native compatibility-shell run completes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeRuntimeArtifacts {
    /// Native startup timing artifact captured for this run, when startup began.
    pub startup_timing: Option<crate::gui_runtime::NativeStartupTimingArtifact>,
    /// Host-defined shutdown artifact captured after the runtime exit hook runs.
    pub shutdown_timing: Option<serde_json::Value>,
}

/// Result plus structured artifacts returned by one native compatibility-shell runtime execution.
pub type NativeRunReport = crate::gui_runtime::RuntimeRunReport<NativeRuntimeArtifacts>;
