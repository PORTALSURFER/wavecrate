use super::WavecrateUiBridge;
use crate::app_core::{
    actions::{
        NativeAppBridge, NativeDirtySegments, NativeFileDropEvent, NativeFileDropPhase,
        NativeFrameBuildResult, NativeMotionModel, NativeSegmentRevisions, NativeUiAction,
    },
    controller::AppControllerUiRuntimeExt,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{error, info};

impl NativeAppBridge for WavecrateUiBridge {
    /// Project the latest app model snapshot as a shared immutable arc.
    fn project_model(&mut self) -> Arc<crate::app_core::actions::NativeAppModel> {
        let model = self.pull_model_arc_snapshot();
        if let Some(recorder) = self.gui_test_recorder.as_mut() {
            recorder.record_projected_model(model.as_ref());
        }
        model
    }

    /// Project the latest app model snapshot by value.
    fn pull_model(&mut self) -> crate::app_core::actions::NativeAppModel {
        Arc::unwrap_or_clone(self.pull_model_arc_snapshot())
    }

    /// Project the latest app model snapshot as a shared immutable arc.
    ///
    /// Returning shared ownership lets retained projection caches reuse model
    /// snapshots across pulls without cloning the full `AppModel`.
    fn pull_model_arc(&mut self) -> Arc<crate::app_core::actions::NativeAppModel> {
        self.pull_model_arc_snapshot()
    }

    /// Return and clear the bridge segment mask from the most recent model pull.
    fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        WavecrateUiBridge::take_dirty_segments(self)
    }

    /// Return the latest static-segment revision snapshot.
    fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        WavecrateUiBridge::take_segment_revisions(self)
    }

    /// Install runtime repaint signal for async job completion wakeups.
    fn install_repaint_signal(&mut self, signal: Arc<dyn radiant::gui::repaint::RepaintSignal>) {
        self.controller.set_repaint_signal(signal);
    }

    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, hwnd: isize) {
        info!(hwnd, "UI bridge: received external drag HWND");
        self.controller
            .set_drag_hwnd(Some(windows::Win32::Foundation::HWND(
                hwnd as *mut std::ffi::c_void,
            )));
    }

    #[cfg(target_os = "windows")]
    fn maybe_launch_external_drag(&mut self, pointer_outside: bool, pointer_left: bool) -> bool {
        let consumed = self
            .controller
            .maybe_launch_external_drag(pointer_outside, pointer_left);
        info!(
            pointer_outside,
            pointer_left, consumed, "UI bridge: external drag poll forwarded to controller"
        );
        consumed
    }

    /// Project motion-only fields for animation-only redraw phases.
    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        WavecrateUiBridge::project_motion_model(self)
    }

    /// Reduce one runtime UI action into controller state.
    fn reduce_action(&mut self, action: NativeUiAction) {
        WavecrateUiBridge::reduce_action(self, action);
    }

    /// Import files dropped by the OS through Radiant's native file-drop hook.
    fn handle_native_file_drop(&mut self, event: NativeFileDropEvent) {
        if event.phase != NativeFileDropPhase::Drop {
            return;
        }
        let Some(path) = event.path else {
            return;
        };
        let target_folder = self
            .controller
            .selected_folder_paths()
            .into_iter()
            .next()
            .unwrap_or_default();
        self.controller
            .import_external_files_to_source_folder(target_folder, vec![path]);
        self.invalidate_projection_key_snapshot();
        self.schedule_full_model_pull_preparation();
    }

    fn take_last_action_handled(&mut self) -> Option<bool> {
        self.last_action_handled.take()
    }

    /// Observe one frame-build result for optional profiling telemetry.
    fn observe_frame_result(&mut self, result: NativeFrameBuildResult) {
        WavecrateUiBridge::observe_frame_result(self, result);
    }

    /// Flush pending work and persist config during runtime shutdown.
    fn on_runtime_exit(&mut self) -> Option<crate::native_runtime::NativeShutdownTimingArtifact> {
        if self.runtime_exit_emitted {
            return None;
        }
        self.runtime_exit_emitted = true;
        let runtime_exit_started = Instant::now();
        let bridge_exit_flush_ms = self.flush_bridge_exit_actions();
        let (config_persist_ms, failure_reason) = self.persist_exit_config();
        let controller_timing = self.controller.request_shutdown_detached_with_timing();
        info!(
            jobs_ms = ms_duration(controller_timing.jobs_shutdown),
            analysis_ms = ms_duration(controller_timing.analysis_shutdown),
            total_ms = ms_duration(controller_timing.total),
            detached = controller_timing.detached,
            "Requested native controller shutdown"
        );
        Some(crate::native_runtime::NativeShutdownTimingArtifact {
            status: exit_status(failure_reason.as_ref(), controller_timing.detached),
            failure_reason,
            bridge_exit_flush_ms: Some(bridge_exit_flush_ms),
            config_persist_ms: Some(config_persist_ms),
            controller_jobs_shutdown_ms: Some(ms_duration(controller_timing.jobs_shutdown)),
            analysis_shutdown_ms: Some(ms_duration(controller_timing.analysis_shutdown)),
            controller_shutdown_ms: Some(ms_duration(controller_timing.total)),
            runtime_exit_total_ms: Some(ms_duration(runtime_exit_started.elapsed())),
        })
    }
}

impl WavecrateUiBridge {
    fn flush_bridge_exit_actions(&mut self) -> f64 {
        let flush_started = Instant::now();
        self.flush_pending_input_actions();
        ms_duration(flush_started.elapsed())
    }

    fn persist_exit_config(&mut self) -> (f64, Option<String>) {
        let config_started = Instant::now();
        let failure_reason = if let Err(err) = self.controller.persist_ui_exit_config() {
            error!(err = %err, "Failed to persist config on native exit");
            Some(String::from("config_persist_failed"))
        } else {
            info!("Persisted config on native exit");
            None
        };
        (ms_duration(config_started.elapsed()), failure_reason)
    }
}

fn exit_status(failure_reason: Option<&String>, detached: bool) -> String {
    if failure_reason.is_some() {
        String::from("error")
    } else if detached {
        String::from("detached")
    } else {
        String::from("complete")
    }
}

fn ms_duration(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
