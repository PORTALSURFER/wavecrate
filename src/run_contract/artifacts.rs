//! Serialized run-contract artifact shapes and timing conversions.

use serde::Serialize;
use wavecrate::gui_runtime::{NativeShutdownTimingArtifact, NativeStartupTimingArtifact};

#[derive(Serialize)]
pub(super) struct RunContractEvent {
    pub(super) run_id: String,
    pub(super) git_sha: String,
    pub(super) persistence_mode: String,
    pub(super) cfg_path: String,
    pub(super) log_path: String,
    pub(super) startup_phase: String,
    pub(super) milestone: String,
    pub(super) exit_status: String,
    pub(super) timestamp_utc: String,
    pub(super) process_id: u32,
    pub(super) manifest_path: String,
    pub(super) artifact_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) startup_timing: Option<RunContractStartupTiming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) shutdown_timing: Option<RunContractShutdownTiming>,
}

#[derive(Clone, Serialize)]
pub(super) struct RunContractMilestone {
    pub(super) name: String,
    pub(super) startup_phase: String,
    pub(super) status: String,
    pub(super) timestamp_utc: String,
}

#[derive(Serialize)]
pub(super) struct RunContractManifest {
    pub(super) run_id: String,
    pub(super) git_sha: String,
    pub(super) persistence_mode: String,
    pub(super) cfg_path: String,
    pub(super) log_path: String,
    pub(super) process_id: u32,
    pub(super) executable_path: String,
    pub(super) working_directory: String,
    pub(super) arg_count: usize,
    pub(super) debug: bool,
    pub(super) started_utc: String,
    pub(super) completed_utc: String,
    pub(super) exit_status: String,
    pub(super) artifact_path: String,
    pub(super) manifest_path: String,
    pub(super) milestones: Vec<RunContractMilestone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) startup_timing: Option<RunContractStartupTiming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) shutdown_timing: Option<RunContractShutdownTiming>,
}

#[derive(Clone, Serialize)]
pub(super) struct RunContractStartupTiming {
    status: String,
    failure_reason: Option<String>,
    window_create_ms: Option<f64>,
    window_revealed_ms: Option<f64>,
    wgpu_surface_create_ms: Option<f64>,
    wgpu_device_ready_ms: Option<f64>,
    surface_ready_ms: Option<f64>,
    renderer_build_ms: Option<f64>,
    renderer_ready_ms: Option<f64>,
    first_scene_ready_ms: Option<f64>,
    first_redraw_started_ms: Option<f64>,
    first_present_draw_ms: Option<f64>,
    first_present_ms: Option<f64>,
    deferred_model_refresh_ms: Option<f64>,
    deferred_model_refresh_total_ms: Option<f64>,
}

#[derive(Clone, Serialize)]
pub(super) struct RunContractShutdownTiming {
    status: String,
    failure_reason: Option<String>,
    bridge_exit_flush_ms: Option<f64>,
    config_persist_ms: Option<f64>,
    controller_jobs_shutdown_ms: Option<f64>,
    analysis_shutdown_ms: Option<f64>,
    controller_shutdown_ms: Option<f64>,
    runtime_exit_total_ms: Option<f64>,
}

impl From<&NativeStartupTimingArtifact> for RunContractStartupTiming {
    fn from(value: &NativeStartupTimingArtifact) -> Self {
        Self {
            status: value.status.clone(),
            failure_reason: value.failure_reason.clone(),
            window_create_ms: value.window_create_ms,
            window_revealed_ms: value.window_revealed_ms,
            wgpu_surface_create_ms: value.wgpu_surface_create_ms,
            wgpu_device_ready_ms: value.wgpu_device_ready_ms,
            surface_ready_ms: value.surface_ready_ms,
            renderer_build_ms: value.renderer_build_ms,
            renderer_ready_ms: value.renderer_ready_ms,
            first_scene_ready_ms: value.first_scene_ready_ms,
            first_redraw_started_ms: value.first_redraw_started_ms,
            first_present_draw_ms: value.first_present_draw_ms,
            first_present_ms: value.first_present_ms,
            deferred_model_refresh_ms: value.deferred_model_refresh_ms,
            deferred_model_refresh_total_ms: value.deferred_model_refresh_total_ms,
        }
    }
}

impl From<&NativeShutdownTimingArtifact> for RunContractShutdownTiming {
    fn from(value: &NativeShutdownTimingArtifact) -> Self {
        Self {
            status: value.status.clone(),
            failure_reason: value.failure_reason.clone(),
            bridge_exit_flush_ms: value.bridge_exit_flush_ms,
            config_persist_ms: value.config_persist_ms,
            controller_jobs_shutdown_ms: value.controller_jobs_shutdown_ms,
            analysis_shutdown_ms: value.analysis_shutdown_ms,
            controller_shutdown_ms: value.controller_shutdown_ms,
            runtime_exit_total_ms: value.runtime_exit_total_ms,
        }
    }
}
