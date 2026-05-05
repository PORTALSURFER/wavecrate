use super::*;
use serde::Serialize;

const STARTUP_PROFILE_ENV: &str = "RADIANT_NATIVE_STARTUP_PROFILE";
const STARTUP_PROFILE_LOG_PREFIX: &str = "[native-vello-startup]";

/// Machine-readable native startup timing payload exported by the runtime.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NativeStartupTimingArtifact {
    /// Whether startup reached the first-present summary path.
    pub status: String,
    /// Explicit startup failure reason when startup exits before first present.
    pub failure_reason: Option<String>,
    /// Milliseconds from startup init to native window creation.
    pub window_create_ms: Option<f64>,
    /// Milliseconds from startup init to window reveal.
    pub window_revealed_ms: Option<f64>,
    /// Milliseconds from window creation to wgpu surface creation.
    pub wgpu_surface_create_ms: Option<f64>,
    /// Milliseconds from window creation to wgpu device readiness.
    pub wgpu_device_ready_ms: Option<f64>,
    /// Milliseconds from startup init to render surface readiness.
    pub surface_ready_ms: Option<f64>,
    /// Milliseconds spent constructing the renderer.
    pub renderer_build_ms: Option<f64>,
    /// Milliseconds from startup init to renderer readiness.
    pub renderer_ready_ms: Option<f64>,
    /// Milliseconds from startup init to first scene readiness.
    pub first_scene_ready_ms: Option<f64>,
    /// Milliseconds from startup init to first redraw start.
    pub first_redraw_started_ms: Option<f64>,
    /// Milliseconds from first redraw start to first present.
    pub first_present_draw_ms: Option<f64>,
    /// Milliseconds from startup init to first present.
    pub first_present_ms: Option<f64>,
    /// Milliseconds between first present and deferred startup refresh completion.
    pub deferred_model_refresh_ms: Option<f64>,
    /// Milliseconds from startup init to deferred startup refresh completion.
    pub deferred_model_refresh_total_ms: Option<f64>,
}

/// Startup lifecycle timing breakdown for first paint and deferred refresh.
#[derive(Debug, Default)]
pub(super) struct StartupTimingProfile {
    enabled: bool,
    init_started_at: Option<Instant>,
    window_created_at: Option<Instant>,
    window_revealed_at: Option<Instant>,
    wgpu_surface_created_at: Option<Instant>,
    wgpu_device_ready_at: Option<Instant>,
    surface_ready_at: Option<Instant>,
    renderer_started_at: Option<Instant>,
    renderer_ready_at: Option<Instant>,
    first_scene_ready_at: Option<Instant>,
    first_redraw_started_at: Option<Instant>,
    first_presented_at: Option<Instant>,
    deferred_model_refresh_done_at: Option<Instant>,
    summary_emitted: bool,
}

impl StartupTimingProfile {
    pub(super) fn new() -> Self {
        let enabled = crate::env_flags::env_var_truthy(STARTUP_PROFILE_ENV);
        Self {
            enabled,
            ..Self::default()
        }
    }

    pub(super) fn mark_init_started(&mut self) {
        self.init_started_at = Some(Instant::now());
    }
    pub(super) fn mark_window_created(&mut self) {
        self.window_created_at = Some(Instant::now());
    }
    pub(super) fn mark_window_revealed(&mut self) {
        self.window_revealed_at.get_or_insert_with(Instant::now);
    }
    pub(super) fn mark_wgpu_surface_created(&mut self) {
        self.wgpu_surface_created_at = Some(Instant::now());
    }
    pub(super) fn mark_wgpu_device_ready(&mut self) {
        self.wgpu_device_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_surface_ready(&mut self) {
        self.surface_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_renderer_started(&mut self) {
        self.renderer_started_at.get_or_insert_with(Instant::now);
    }
    pub(super) fn mark_renderer_ready(&mut self) {
        self.renderer_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_first_scene_ready(&mut self) {
        self.first_scene_ready_at = Some(Instant::now());
    }
    pub(super) fn mark_first_redraw_started(&mut self) {
        self.first_redraw_started_at
            .get_or_insert_with(Instant::now);
    }
    pub(super) fn mark_first_presented(&mut self) {
        self.first_presented_at = Some(Instant::now());
    }
    pub(super) fn mark_deferred_model_refresh_done(&mut self) {
        self.deferred_model_refresh_done_at = Some(Instant::now());
    }

    pub(super) fn maybe_emit_summary(&mut self) {
        if self.summary_emitted {
            return;
        }
        let Some(artifact) = self.export_completed_artifact() else {
            return;
        };
        info!(
            window_create_ms = artifact.window_create_ms.unwrap_or_default(),
            window_revealed_ms = artifact.window_revealed_ms.unwrap_or_default(),
            wgpu_surface_create_ms = artifact.wgpu_surface_create_ms.unwrap_or_default(),
            wgpu_device_ready_ms = artifact.wgpu_device_ready_ms.unwrap_or_default(),
            surface_ready_ms = artifact.surface_ready_ms.unwrap_or_default(),
            renderer_build_ms = artifact.renderer_build_ms.unwrap_or_default(),
            renderer_ready_ms = artifact.renderer_ready_ms.unwrap_or_default(),
            first_scene_ready_ms = artifact.first_scene_ready_ms.unwrap_or_default(),
            first_redraw_started_ms = artifact.first_redraw_started_ms.unwrap_or_default(),
            first_present_draw_ms = artifact.first_present_draw_ms.unwrap_or_default(),
            first_present_ms = artifact.first_present_ms.unwrap_or_default(),
            deferred_model_refresh_ms = artifact.deferred_model_refresh_ms.unwrap_or_default(),
            deferred_model_refresh_total_ms =
                artifact.deferred_model_refresh_total_ms.unwrap_or_default(),
            "native vello startup timing summary"
        );
        if self.enabled {
            eprintln!(
                "{STARTUP_PROFILE_LOG_PREFIX} window_create_ms={:.3} \
window_revealed_ms={:.3} \
wgpu_surface_create_ms={:.3} \
wgpu_device_ready_ms={:.3} \
surface_ready_ms={:.3} renderer_ready_ms={:.3} \
renderer_build_ms={:.3} first_scene_ready_ms={:.3} \
first_redraw_started_ms={:.3} \
first_present_draw_ms={:.3} first_present_ms={:.3} \
deferred_model_refresh_ms={:.3} \
deferred_model_refresh_total_ms={:.3}",
                artifact.window_create_ms.unwrap_or_default(),
                artifact.window_revealed_ms.unwrap_or_default(),
                artifact.wgpu_surface_create_ms.unwrap_or_default(),
                artifact.wgpu_device_ready_ms.unwrap_or_default(),
                artifact.surface_ready_ms.unwrap_or_default(),
                artifact.renderer_ready_ms.unwrap_or_default(),
                artifact.renderer_build_ms.unwrap_or_default(),
                artifact.first_scene_ready_ms.unwrap_or_default(),
                artifact.first_redraw_started_ms.unwrap_or_default(),
                artifact.first_present_draw_ms.unwrap_or_default(),
                artifact.first_present_ms.unwrap_or_default(),
                artifact.deferred_model_refresh_ms.unwrap_or_default(),
                artifact.deferred_model_refresh_total_ms.unwrap_or_default(),
            );
        }
        self.summary_emitted = true;
    }

    pub(super) fn export_artifact(&self) -> Option<NativeStartupTimingArtifact> {
        self.export_completed_artifact()
            .or_else(|| self.export_incomplete_artifact())
    }

    fn export_completed_artifact(&self) -> Option<NativeStartupTimingArtifact> {
        let (Some(init_started_at), Some(window_created_at), Some(first_presented_at)) = (
            self.init_started_at,
            self.window_created_at,
            self.first_presented_at,
        ) else {
            return None;
        };
        let surface_ready_at = self.surface_ready_at.unwrap_or(first_presented_at);
        let renderer_ready_at = self.renderer_ready_at.unwrap_or(first_presented_at);
        let first_scene_ready_at = self.first_scene_ready_at.unwrap_or(first_presented_at);
        let deferred_model_refresh_done_at = self
            .deferred_model_refresh_done_at
            .unwrap_or(first_presented_at);
        let window_create_ms = Some(ms_between(init_started_at, window_created_at));
        let first_present_ms = Some(ms_between(init_started_at, first_presented_at));

        Some(NativeStartupTimingArtifact {
            status: String::from("complete"),
            failure_reason: None,
            window_create_ms,
            window_revealed_ms: Some(
                self.window_revealed_at
                    .map(|at| ms_between(init_started_at, at))
                    .unwrap_or(first_present_ms.unwrap_or_default()),
            ),
            wgpu_surface_create_ms: Some(
                self.wgpu_surface_created_at
                    .map(|at| ms_between(window_created_at, at))
                    .unwrap_or(0.0),
            ),
            wgpu_device_ready_ms: Some(
                self.wgpu_device_ready_at
                    .map(|at| ms_between(window_created_at, at))
                    .unwrap_or(0.0),
            ),
            surface_ready_ms: Some(ms_between(init_started_at, surface_ready_at)),
            renderer_build_ms: Some(
                self.renderer_started_at
                    .map(|at| ms_between(at, renderer_ready_at))
                    .unwrap_or(0.0),
            ),
            renderer_ready_ms: Some(ms_between(init_started_at, renderer_ready_at)),
            first_scene_ready_ms: Some(ms_between(init_started_at, first_scene_ready_at)),
            first_redraw_started_ms: Some(
                self.first_redraw_started_at
                    .map(|at| ms_between(init_started_at, at))
                    .unwrap_or_else(|| ms_between(init_started_at, first_scene_ready_at)),
            ),
            first_present_draw_ms: Some(
                self.first_redraw_started_at
                    .map(|at| ms_between(at, first_presented_at))
                    .unwrap_or(0.0),
            ),
            first_present_ms,
            deferred_model_refresh_ms: Some(ms_between(
                first_presented_at,
                deferred_model_refresh_done_at,
            )),
            deferred_model_refresh_total_ms: Some(ms_between(
                init_started_at,
                deferred_model_refresh_done_at,
            )),
        })
    }

    fn export_incomplete_artifact(&self) -> Option<NativeStartupTimingArtifact> {
        let init_started_at = self.init_started_at?;
        let status = self.failure_reason()?;
        let window_created_at = self.window_created_at;
        let renderer_ready_at = self.renderer_ready_at;

        Some(NativeStartupTimingArtifact {
            status: String::from("incomplete"),
            failure_reason: Some(status.to_string()),
            window_create_ms: window_created_at.map(|at| ms_between(init_started_at, at)),
            window_revealed_ms: self
                .window_revealed_at
                .map(|at| ms_between(init_started_at, at)),
            wgpu_surface_create_ms: window_created_at.and_then(|window_created_at| {
                self.wgpu_surface_created_at
                    .map(|at| ms_between(window_created_at, at))
            }),
            wgpu_device_ready_ms: window_created_at.and_then(|window_created_at| {
                self.wgpu_device_ready_at
                    .map(|at| ms_between(window_created_at, at))
            }),
            surface_ready_ms: self
                .surface_ready_at
                .map(|at| ms_between(init_started_at, at)),
            renderer_build_ms: self
                .renderer_started_at
                .zip(renderer_ready_at)
                .map(|(started_at, ready_at)| ms_between(started_at, ready_at)),
            renderer_ready_ms: renderer_ready_at.map(|at| ms_between(init_started_at, at)),
            first_scene_ready_ms: self
                .first_scene_ready_at
                .map(|at| ms_between(init_started_at, at)),
            first_redraw_started_ms: self
                .first_redraw_started_at
                .map(|at| ms_between(init_started_at, at)),
            first_present_draw_ms: None,
            first_present_ms: None,
            deferred_model_refresh_ms: None,
            deferred_model_refresh_total_ms: None,
        })
    }

    /// Return the explicit startup-profile failure reason for a run that exited
    /// before first present, if startup had already begun.
    fn failure_reason(&self) -> Option<&'static str> {
        if self.summary_emitted
            || self.first_presented_at.is_some()
            || self.init_started_at.is_none()
        {
            return None;
        }
        Some("startup_exited_before_first_present")
    }

    fn emit_failure_reason_if_needed(&self) {
        let Some(reason) = self.failure_reason() else {
            return;
        };
        if self.enabled {
            eprintln!("{STARTUP_PROFILE_LOG_PREFIX} status=failed reason={reason}");
        }
    }

    #[cfg(test)]
    pub(super) fn did_emit_summary(&self) -> bool {
        self.summary_emitted
    }

    #[cfg(test)]
    pub(super) fn failure_reason_for_test(&self) -> Option<&'static str> {
        self.failure_reason()
    }
}

impl Drop for StartupTimingProfile {
    fn drop(&mut self) {
        self.emit_failure_reason_if_needed();
    }
}

fn ms_between(start: Instant, end: Instant) -> f64 {
    (end - start).as_secs_f64() * 1000.0
}
