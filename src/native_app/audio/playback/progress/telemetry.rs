use std::time::Duration;

use super::super::diagnostics::PlayheadFrameMessageDiagnostics;
use crate::native_app::{
    app::{NativeAppState, SamplePlaybackSession},
    starmap_audition_telemetry::{
        self as starmap_telemetry, StarmapAuditionCounter, StarmapAuditionDuration,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct FrameRepaintScopeSnapshot {
    playing: bool,
    playback_visual_generation: u64,
    play_selection_flash_active: bool,
    copy_flash_frames: u8,
    protected_source_error_flash_frames: u8,
    drag_hover_auto_expand_pending: bool,
    folder_progress_active: bool,
    normalization_progress_active: bool,
    file_move_progress_active: bool,
    source_cache_progress_active: bool,
    waveform_loading_active: bool,
    sample_loading: bool,
    audio_opening: bool,
    audio_settings_error_active: bool,
    audio_output_sample_rate: Option<u32>,
    startup_source_scan_pending: bool,
    startup_auto_load_pending: bool,
    pending_playback_start: bool,
}

impl NativeAppState {
    pub(in crate::native_app) fn frame_repaint_scope_before_update(
        &self,
    ) -> FrameRepaintScopeSnapshot {
        FrameRepaintScopeSnapshot::from_state(self)
    }

    pub(in crate::native_app) fn frame_can_use_paint_only(
        &mut self,
        before: FrameRepaintScopeSnapshot,
    ) -> bool {
        let after = FrameRepaintScopeSnapshot::from_state(self);
        let same_transient_frame_state = before.same_transient_frame_state(after);
        let requires_surface_frame = before.requires_surface_frame();
        let paint_only = same_transient_frame_state && !requires_surface_frame;
        if before.playing || after.playing {
            self.playhead_frame_diagnostics
                .record_frame_message(PlayheadFrameMessageDiagnostics {
                    paint_only,
                    reason: frame_repaint_reason(
                        same_transient_frame_state,
                        requires_surface_frame,
                    ),
                });
        }
        paint_only
    }

    pub(in crate::native_app) fn observe_playhead_native_frame_diagnostics(
        &mut self,
        diagnostics: radiant::runtime::NativeFrameDiagnostics,
    ) {
        self.playhead_frame_diagnostics
            .observe_native_frame(diagnostics);
    }
}

pub(super) fn log_runtime_playback_event(
    stage: &'static str,
    outcome: &'static str,
    starmap_counter: Option<StarmapAuditionCounter>,
    session: &SamplePlaybackSession,
    elapsed: Option<Duration>,
    error: Option<&str>,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    tracing::info!(
        target: "perf::audio_start",
        module = "wavecrate_native_playback",
        stage,
        outcome,
        origin = session.request.origin,
        source_kind = session.source_kind,
        path = %session.request.path,
        start_ratio = session.request.span.0,
        end_ratio = session.request.span.1,
        elapsed_ms = elapsed.map(duration_ms).unwrap_or(0.0),
        error = error.unwrap_or_default(),
        "Native playback runtime event"
    );
    if session.request.origin != "starmap_drag" {
        return;
    }
    if let Some(elapsed) = elapsed {
        starmap_telemetry::record_duration(StarmapAuditionDuration::RuntimeStart, elapsed);
    }
    starmap_telemetry::record_event(
        starmap_counter,
        stage,
        outcome,
        Some(session.request.path.as_str()),
        0,
        0,
        true,
        elapsed,
    );
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

impl FrameRepaintScopeSnapshot {
    fn from_state(state: &NativeAppState) -> Self {
        Self {
            playing: state.playback_visual_activity_active(),
            playback_visual_generation: state.waveform.current.playback_visual_generation(),
            play_selection_flash_active: state.waveform.current.play_selection_flash_active(),
            copy_flash_frames: state
                .library
                .folder_browser
                .copy_flash_frames()
                .max(state.waveform.current.copy_flash_frames()),
            protected_source_error_flash_frames: state
                .library
                .folder_browser
                .protected_source_error_flash_frames()
                .max(state.waveform.current.protected_source_error_flash_frames()),
            drag_hover_auto_expand_pending: state
                .library
                .folder_browser
                .drag_hover_auto_expand_pending(),
            folder_progress_active: state.library.folder_scan_active(),
            normalization_progress_active: state.background.normalization_progress.is_some(),
            file_move_progress_active: state.background.file_move_progress.is_some(),
            source_cache_progress_active: state
                .waveform
                .cache
                .active_folder_warm_folder_id
                .is_some(),
            waveform_loading_active: state.waveform.load.label.is_some(),
            sample_loading: state.active_sample_load_task().is_some(),
            audio_opening: state.background.audio_open.active().is_some(),
            audio_settings_error_active: state.audio.settings_error.is_some(),
            audio_output_sample_rate: state
                .audio
                .output_resolved
                .as_ref()
                .map(|output| output.sample_rate),
            startup_source_scan_pending: state.ui.startup.source_scan_pending,
            startup_auto_load_pending: state.ui.startup.auto_load_pending,
            pending_playback_start: state.audio.pending_playback_start.is_some(),
        }
    }

    fn requires_surface_frame(self) -> bool {
        self.play_selection_flash_active
            || self.copy_flash_frames > 0
            || self.protected_source_error_flash_frames > 0
            || self.folder_progress_active
            || self.file_move_progress_active
            || self.source_cache_progress_active
            || self.sample_loading
            || self.audio_opening
            || self.startup_source_scan_pending
            || self.startup_auto_load_pending
            || self.pending_playback_start
    }

    fn same_transient_frame_state(self, after: Self) -> bool {
        self.playing == after.playing
            && self.playback_visual_generation == after.playback_visual_generation
            && self.play_selection_flash_active == after.play_selection_flash_active
            && self.copy_flash_frames == after.copy_flash_frames
            && self.protected_source_error_flash_frames == after.protected_source_error_flash_frames
            && self.drag_hover_auto_expand_pending == after.drag_hover_auto_expand_pending
            && self.folder_progress_active == after.folder_progress_active
            && self.normalization_progress_active == after.normalization_progress_active
            && self.file_move_progress_active == after.file_move_progress_active
            && self.source_cache_progress_active == after.source_cache_progress_active
            && self.waveform_loading_active == after.waveform_loading_active
            && self.sample_loading == after.sample_loading
            && self.audio_opening == after.audio_opening
            && self.audio_settings_error_active == after.audio_settings_error_active
            && self.audio_output_sample_rate == after.audio_output_sample_rate
            && self.startup_source_scan_pending == after.startup_source_scan_pending
            && self.startup_auto_load_pending == after.startup_auto_load_pending
            && self.pending_playback_start == after.pending_playback_start
    }
}

fn frame_repaint_reason(
    same_transient_frame_state: bool,
    requires_surface_frame: bool,
) -> &'static str {
    if requires_surface_frame {
        "surface_frame_required"
    } else if !same_transient_frame_state {
        "transient_state_changed"
    } else {
        "paint_only"
    }
}
