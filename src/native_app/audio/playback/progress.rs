use std::{
    path::Path,
    time::{Duration, Instant},
};

use super::PLAYBACK_START_ACTIVE_SOURCE_GRACE;
use crate::native_app::app::{
    NativeAppState, SamplePlaybackSession, SamplePlaybackSessionState, emit_gui_action,
    sample_path_label,
};
use crate::native_app::app_chrome::library_browser::sample_browser_view;
use crate::native_app::starmap_audition_telemetry::{
    self as starmap_telemetry, StarmapAuditionCounter, StarmapAuditionDuration,
};
use crate::native_app::ui::ids::SAMPLE_BROWSER_MAP_ID;
use crate::native_app::waveform::{WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID};
use radiant::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintPrimitive, TransientOverlayContext, WidgetPaint},
};
use wavecrate::audio::{
    PlaybackRuntimeCancellation, PlaybackRuntimeEvent, PlaybackRuntimeProgress,
    PlaybackRuntimeStarted,
};

const PLAYBACK_CURSOR_COLOR: Rgba8 = Rgba8 {
    r: 71,
    g: 220,
    b: 255,
    a: 245,
};
const PLAYBACK_CURSOR_WIDTH: f32 = 2.0;
const LOADING_BACKGROUND_COLOR: Rgba8 = Rgba8 {
    r: 22,
    g: 24,
    b: 25,
    a: 72,
};
const LOADING_PROGRESS_COLOR: Rgba8 = Rgba8 {
    r: 174,
    g: 178,
    b: 181,
    a: 118,
};
const AUDIO_OUTPUT_STREAM_ERROR_PREFIX: &str = "Audio output stream error:";
const AUDIO_OUTPUT_UNAVAILABLE_ERROR: &str = "Audio output stream is unavailable";

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
    pub(in crate::native_app) fn runtime_playback_origin_for_path(
        &self,
        path: &str,
    ) -> &'static str {
        if self.ui.chrome.starmap_audition_drag.is_some()
            || self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .as_deref()
                == Some(path)
        {
            "starmap_drag"
        } else if self.audio.active_sample_playback_matches(path) {
            "instant_audition"
        } else {
            "browser"
        }
    }

    pub(in crate::native_app) fn current_waveform_runtime_source_kind(&self) -> &'static str {
        if self.waveform.current.playback_samples().is_some() {
            "decoded_samples"
        } else if self.waveform.current.playback_cache_file().is_some() {
            "interleaved_f32_file"
        } else if self.waveform.current.playback_source_file().is_some() {
            "audio_file"
        } else {
            "audio_bytes"
        }
    }

    pub(in crate::native_app) fn frame_repaint_scope_before_update(
        &self,
    ) -> FrameRepaintScopeSnapshot {
        FrameRepaintScopeSnapshot::from_state(self)
    }

    pub(in crate::native_app) fn frame_can_use_paint_only(
        &self,
        before: FrameRepaintScopeSnapshot,
    ) -> bool {
        let after = FrameRepaintScopeSnapshot::from_state(self);
        before.same_transient_frame_state(after) && !before.requires_surface_frame()
    }

    pub(in crate::native_app) fn sync_edit_fade_audio_state(&mut self) {
        if let Some(player) = self.audio.player.as_ref() {
            player.set_edit_fade_state(wavecrate::audio::edit_fade_range_from_selection(
                self.waveform.current.edit_selection(),
            ));
        }
    }

    pub(in crate::native_app) fn drain_playback_runtime_events(&mut self) {
        let Some(events) = self.audio.playback_events.take() else {
            return;
        };
        for event in events.try_iter() {
            self.apply_playback_runtime_event(event);
        }
        if self.audio.playback_runtime.is_some() {
            self.audio.playback_events = Some(events);
        }
    }

    fn apply_playback_runtime_event(&mut self, event: PlaybackRuntimeEvent) {
        match event {
            PlaybackRuntimeEvent::Started(started) => self.finish_runtime_playback_started(started),
            PlaybackRuntimeEvent::Failed { id, error } => {
                self.finish_runtime_playback_failed(id, error)
            }
            PlaybackRuntimeEvent::Cancelled { id, reason } => {
                self.finish_runtime_playback_cancelled(id, reason)
            }
            PlaybackRuntimeEvent::Stopped { .. } => {}
            PlaybackRuntimeEvent::Progress { progress, .. } => {
                self.apply_runtime_playback_progress(progress)
            }
        }
    }

    fn apply_runtime_playback_progress(&mut self, progress: PlaybackRuntimeProgress) {
        if self.audio.active_sample_playback_pending_runtime() {
            return;
        }
        self.audio.set_playback_progress(progress);
    }

    fn finish_runtime_playback_started(&mut self, started: PlaybackRuntimeStarted) {
        let Some(session) = self.audio.sample_playback_session.as_mut() else {
            return;
        };
        if session.runtime_request_id != Some(started.id) {
            log_runtime_playback_event(
                "runtime.started",
                "id_mismatch",
                Some(StarmapAuditionCounter::RuntimeStale),
                session,
                None,
                None,
            );
            return;
        }
        let submit_elapsed = session.submitted_at.elapsed();
        log_runtime_playback_event(
            "runtime.started",
            "started",
            Some(StarmapAuditionCounter::RuntimeStarted),
            session,
            Some(submit_elapsed),
            None,
        );
        let source_updates_waveform = session.request.visibility.updates_waveform_playhead();
        session.state = if source_updates_waveform {
            SamplePlaybackSessionState::WaveformVisible
        } else {
            SamplePlaybackSessionState::AudibleTransient
        };
        let path = session.request.path.clone();
        let span = session.request.span;
        let show_start_marker = session.request.show_start_marker;
        self.audio.output_resolved = Some(started.output);
        self.audio.current_playback_span = source_updates_waveform.then_some(span);
        self.audio
            .set_started_playback_progress(wavecrate::audio::PlaybackRuntimeProgress {
                active: true,
                elapsed: Some(Duration::ZERO),
                looping: self.audio.loop_playback,
                progress: Some(started.playback_start),
                error: None,
            });
        if source_updates_waveform && self.waveform.current.path() == Path::new(&path) {
            if show_start_marker {
                self.waveform.current.start_playback(started.playback_start);
            } else {
                self.waveform
                    .current
                    .start_playback_without_marker(started.playback_start);
            }
        }
        self.ui.status.sample = format!("Playing {}", sample_path_label(&path));
    }

    fn finish_runtime_playback_failed(
        &mut self,
        id: wavecrate::audio::PlaybackRequestId,
        error: String,
    ) {
        let Some(session) = self.audio.sample_playback_session.as_mut() else {
            return;
        };
        if session.runtime_request_id != Some(id) {
            log_runtime_playback_event(
                "runtime.failed",
                "id_mismatch",
                Some(StarmapAuditionCounter::RuntimeStale),
                session,
                None,
                Some(&error),
            );
            return;
        }
        let submit_elapsed = session.submitted_at.elapsed();
        log_runtime_playback_event(
            "runtime.failed",
            "failed",
            Some(StarmapAuditionCounter::RuntimeFailed),
            session,
            Some(submit_elapsed),
            Some(&error),
        );
        if playback_error_indicates_output_unavailable(&error) {
            self.mark_audio_output_unavailable(error);
            return;
        }
        let path = session.request.path.clone();
        session.state = SamplePlaybackSessionState::Failed(error.clone());
        self.audio.clear_sample_playback_session();
        self.audio.current_playback_span = None;
        self.waveform.current.stop_playback();
        self.ui.status.sample = format!(
            "Loaded {} | playback unavailable: {error}",
            sample_path_label(&path)
        );
    }

    fn finish_runtime_playback_cancelled(
        &mut self,
        id: wavecrate::audio::PlaybackRequestId,
        reason: PlaybackRuntimeCancellation,
    ) {
        let Some(session) = self.audio.sample_playback_session.as_ref() else {
            return;
        };
        if session.runtime_request_id != Some(id) {
            log_runtime_playback_event(
                "runtime.cancelled",
                "id_mismatch",
                Some(StarmapAuditionCounter::RuntimeStale),
                session,
                None,
                None,
            );
            return;
        }
        let submit_elapsed = session.submitted_at.elapsed();
        log_runtime_playback_event(
            "runtime.cancelled",
            match reason {
                PlaybackRuntimeCancellation::Superseded => "superseded",
                PlaybackRuntimeCancellation::Stopped => "stopped",
                PlaybackRuntimeCancellation::Shutdown => "shutdown",
            },
            Some(StarmapAuditionCounter::RuntimeCancelled),
            session,
            Some(submit_elapsed),
            None,
        );
        if reason != PlaybackRuntimeCancellation::Superseded {
            self.audio.clear_sample_playback_session();
            self.audio.current_playback_span = None;
            self.waveform.current.stop_playback();
        }
    }

    pub(in crate::native_app) fn refresh_playback_progress(&mut self) {
        if let Some(runtime) = self.audio.playback_runtime.as_ref() {
            let _ = runtime.try_poll_progress();
        }
        if self.audio.playback_runtime.is_some() {
            self.refresh_runtime_playback_progress();
            return;
        }
        let Some(player) = self.audio.player.as_mut() else {
            return;
        };
        if let Some(error) = player.take_error() {
            self.stop_playback_after_progress_error(error);
            return;
        }

        let active = player.is_playing();
        let elapsed = player.playback_elapsed();
        let player_looping = player.is_looping();
        let progress = player.progress();
        let should_be_looping = self.audio.loop_playback && self.waveform.current.is_playing();
        let within_start_grace =
            elapsed.is_some_and(|elapsed| elapsed <= PLAYBACK_START_ACTIVE_SOURCE_GRACE);

        if self.loop_recovery_needed(
            should_be_looping,
            player_looping,
            active,
            within_start_grace,
        ) {
            self.recover_progress_loop_playback(player_looping);
            return;
        }

        if active || within_start_grace || (should_be_looping && player_looping) {
            if let Some(progress) = progress {
                self.waveform.current.set_playhead_ratio(progress);
            }
        } else if self.waveform.current.is_playing() {
            self.finish_playback_progress();
        }
    }

    fn refresh_runtime_playback_progress(&mut self) {
        if let Some(error) = self.audio.playback_progress.error.take() {
            self.stop_playback_after_progress_error(error);
            return;
        }
        if self.audio.active_sample_playback_pending_runtime() {
            return;
        }

        let active = self.audio.playback_progress.active;
        let elapsed = self.audio.playback_progress.elapsed;
        let player_looping = self.audio.playback_progress.looping;
        let progress = self.audio.playback_progress.progress;
        let should_be_looping = self.audio.loop_playback && self.waveform.current.is_playing();
        let within_start_grace =
            elapsed.is_some_and(|elapsed| elapsed <= PLAYBACK_START_ACTIVE_SOURCE_GRACE);
        if self
            .audio
            .sample_playback_session
            .as_ref()
            .is_some_and(|session| {
                session.source_kind == "preview_samples"
                    && !session.request.visibility.updates_waveform_playhead()
            })
        {
            return;
        }

        if self.loop_recovery_needed(
            should_be_looping,
            player_looping,
            active,
            within_start_grace,
        ) {
            self.recover_progress_loop_playback(player_looping);
            return;
        }

        if active || within_start_grace || (should_be_looping && player_looping) {
            if let Some(progress) = progress {
                self.waveform.current.set_playhead_ratio(progress);
            }
        } else if self.waveform.current.is_playing() {
            self.finish_playback_progress();
        }
    }

    pub(in crate::native_app) fn paint_playback_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if self.chrome_overlay_suppresses_waveform_transient_overlay() {
            return;
        }
        let Some(progress) = self.current_audio_progress_ratio_for_frame(context.animation_time)
        else {
            return;
        };
        let Some(visible_ratio) = self.waveform.current.visible_ratio_for_absolute(progress) else {
            return;
        };
        let Some(bounds) = context
            .plan
            .first_widget_rect_by_priority([WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID])
        else {
            return;
        };
        push_playback_cursor(primitives, bounds, visible_ratio);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn paint_waveform_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if self.chrome_overlay_suppresses_waveform_transient_overlay() {
            return;
        }
        self.paint_loading_overlay(context, primitives);
        self.paint_playback_overlay(context, primitives);
    }

    pub(in crate::native_app) fn paint_app_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if self.chrome_overlay_suppresses_waveform_transient_overlay() {
            return;
        }
        self.paint_loading_overlay(context, primitives);
        self.paint_playback_overlay(context, primitives);
        self.paint_starmap_active_audition_overlay(context, primitives);
    }

    pub(in crate::native_app) fn should_paint_app_transient_overlay(&self) -> bool {
        !self.chrome_overlay_suppresses_waveform_transient_overlay()
            && (self.playback_visual_activity_active()
                || self.waveform.load.label.is_some()
                || self.active_starmap_audition_file_id().is_some())
    }

    #[cfg(test)]
    pub(in crate::native_app) fn should_paint_waveform_transient_overlay(&self) -> bool {
        !self.chrome_overlay_suppresses_waveform_transient_overlay()
            && (self.playback_visual_activity_active() || self.waveform.load.label.is_some())
    }

    pub(in crate::native_app) fn playback_visual_activity_active(&self) -> bool {
        self.waveform.current.is_playing()
            || self.audio.sample_playback_session.is_some()
            || self.audio.playback_progress.active
    }

    fn chrome_overlay_suppresses_waveform_transient_overlay(&self) -> bool {
        self.ui.chrome.shortcut_help_open
            || self.ui.chrome.transaction_list_open
            || self.ui.browser_interaction.context_menu.is_some()
            || self.ui.browser_interaction.waveform_context_menu.is_some()
            || self
                .library
                .folder_browser
                .pending_file_move_conflict_view()
                .is_some()
            || self.ui.browser_interaction.pending_folder_delete.is_some()
            || self
                .ui
                .browser_interaction
                .pending_waveform_destructive_edit
                .is_some()
    }

    fn paint_loading_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if self.waveform.load.label.is_none() {
            return;
        }
        let Some(bounds) = context
            .plan
            .first_widget_rect_by_priority([WAVEFORM_WIDGET_ID, WAVEFORM_SIGNAL_WIDGET_ID])
        else {
            return;
        };
        let mut paint = WidgetPaint::new(primitives, WAVEFORM_WIDGET_ID);
        paint.push_visible_fill_rect(bounds, LOADING_BACKGROUND_COLOR);
        paint.push_horizontal_progress_fill(
            bounds,
            self.waveform.load.progress,
            LOADING_PROGRESS_COLOR,
        );
    }

    fn paint_starmap_active_audition_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(active_file_id) = self.active_starmap_audition_file_id() else {
            return;
        };
        let Some(bounds) = context
            .plan
            .first_widget_rect_by_priority([SAMPLE_BROWSER_MAP_ID])
        else {
            return;
        };
        let Some(items) = self.library.folder_browser.cached_starmap_projection() else {
            return;
        };
        sample_browser_view::paint_active_starmap_audition_overlay(
            primitives,
            bounds,
            &items,
            self.ui.chrome.starmap_viewport,
            active_file_id,
        );
    }

    fn active_starmap_audition_file_id(&self) -> Option<&str> {
        self.ui
            .chrome
            .starmap_audition_drag
            .as_ref()
            .and_then(|drag| drag.last_hit_file_id.as_deref())
            .or(self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .as_deref())
            .or_else(|| {
                self.audio
                    .sample_playback_session
                    .as_ref()
                    .filter(|session| session.request.origin == "starmap_drag")
                    .map(|session| session.request.path.as_str())
            })
    }

    fn stop_playback_after_progress_error(&mut self, error: String) {
        if playback_error_indicates_output_unavailable(&error) {
            self.mark_audio_output_unavailable(error);
            return;
        }
        let started_at = Instant::now();
        self.waveform.current.stop_playback();
        self.ui.status.sample = format!("Playback stopped: {error}");
        emit_gui_action(
            "playback.progress",
            Some("transport"),
            None,
            "error",
            started_at,
            Some(&error),
        );
    }

    pub(in crate::native_app) fn mark_audio_output_unavailable(&mut self, error: String) {
        let started_at = Instant::now();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.pending_playback_start = None;
        self.audio.clear_sample_playback_session();
        if let Some(runtime) = self.audio.playback_runtime.take() {
            let _ = runtime.try_shutdown();
        }
        self.audio.player = None;
        self.audio.playback_events = None;
        self.audio.clear_playback_progress();
        self.audio.output_resolved = None;
        self.audio.settings_error = Some(error.clone());
        self.ui.status.sample = format!("Audio output OFF: {error}");
        emit_gui_action(
            "audio.output.runtime",
            Some("audio"),
            None,
            "offline",
            started_at,
            Some(&error),
        );
    }

    fn loop_recovery_needed(
        &self,
        should_be_looping: bool,
        player_looping: bool,
        active: bool,
        within_start_grace: bool,
    ) -> bool {
        should_be_looping && (!player_looping || (!active && !within_start_grace))
    }

    fn recover_progress_loop_playback(&mut self, player_looping: bool) {
        let reason = if !player_looping {
            "player_not_looping"
        } else {
            "loop_source_inactive"
        };
        if let Err(err) = self.recover_loop_playback(reason) {
            self.audio.loop_playback = false;
            self.waveform.current.stop_playback();
            self.audio.current_playback_span = None;
            self.audio.clear_sample_playback_session();
            self.ui.status.sample = format!("Loop playback stopped: {err}");
            emit_gui_action(
                "playback.loop.recover",
                Some("transport"),
                None,
                "error",
                Instant::now(),
                Some(&err),
            );
        }
    }

    fn finish_playback_progress(&mut self) {
        let started_at = Instant::now();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.clear_playback_progress();
        self.audio.clear_sample_playback_session();
        emit_gui_action(
            "playback.progress",
            Some("transport"),
            None,
            "completed",
            started_at,
            None,
        );
    }
}

fn log_runtime_playback_event(
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

fn push_playback_cursor(primitives: &mut Vec<PaintPrimitive>, bounds: Rect, ratio: f32) {
    let width = PLAYBACK_CURSOR_WIDTH
        .ceil()
        .clamp(1.0, bounds.width().max(1.0));
    let center_x = bounds.x_for_ratio(ratio.clamp(0.0, 1.0));
    let left =
        (center_x - width * 0.5).clamp(bounds.min.x, (bounds.max.x - width).max(bounds.min.x));
    let right = (left + width).min(bounds.max.x);
    if right <= left {
        return;
    }
    WidgetPaint::new(primitives, WAVEFORM_WIDGET_ID).push_visible_fill_rect(
        Rect::from_min_max(
            Point::new(left, bounds.min.y),
            Point::new(right, bounds.max.y),
        ),
        PLAYBACK_CURSOR_COLOR,
    );
}

fn playback_error_indicates_output_unavailable(error: &str) -> bool {
    error.starts_with(AUDIO_OUTPUT_STREAM_ERROR_PREFIX)
        || error.contains(AUDIO_OUTPUT_UNAVAILABLE_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::{
        app::{
            SamplePlaybackHistory, SamplePlaybackIntent, SamplePlaybackRequest,
            SamplePlaybackSessionState, SamplePlaybackSourceProbe,
        },
        test_support::state::{NativeAppStateFixture, WaveformState},
        waveform::test_decoded_waveform_file_from_mono_samples,
    };
    use std::{path::PathBuf, sync::Arc};

    #[test]
    fn preview_slice_runtime_progress_does_not_move_full_waveform_playhead() {
        let path = PathBuf::from("preview-visual.wav");
        let file =
            test_decoded_waveform_file_from_mono_samples(path.clone(), vec![0.0, 0.5, -0.5, 0.0]);
        let mut state = NativeAppStateFixture::default().build();
        state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
        let request = SamplePlaybackRequest::transient(
            path.display().to_string(),
            SamplePlaybackIntent::TransientNavigation,
            "browser",
        )
        .with_source_probe(SamplePlaybackSourceProbe::CachedOnly);
        state
            .audio
            .start_resolving_sample_playback_session(request, "preview_samples");
        state.audio.playback_progress = wavecrate::audio::PlaybackRuntimeProgress {
            active: true,
            elapsed: Some(Duration::from_millis(80)),
            looping: false,
            progress: Some(0.5),
            error: None,
        };

        state.refresh_runtime_playback_progress();

        assert!(
            !state.waveform.current.is_playing(),
            "preview-slice playback should not mark the full waveform as playing"
        );
        assert_eq!(
            state.waveform.current.playhead_ratio(),
            None,
            "preview-slice progress is normalized to the tiny cache slice, not the full waveform"
        );
        assert_eq!(state.audio.current_playback_span, None);
        assert_eq!(state.audio.playback_progress.progress, Some(0.5));
    }

    #[test]
    fn pending_runtime_restart_ignores_progress_until_started_event() {
        let path = PathBuf::from("restart-visual.wav");
        let file = test_decoded_waveform_file_from_mono_samples(
            path.clone(),
            vec![0.0, 0.5, -0.5, 0.0, 0.25, -0.25],
        );
        let mut state = NativeAppStateFixture::default().build();
        state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
        state.waveform.current.start_playback(0.1);
        state.audio.current_playback_span = Some((0.1, 0.9));
        let request = SamplePlaybackRequest::waveform(
            path.display().to_string(),
            (0.1, 0.9),
            SamplePlaybackIntent::WaveformSpan,
            "waveform",
            SamplePlaybackHistory::Record,
        );
        state
            .audio
            .start_resolving_sample_playback_session(request, "decoded_samples");
        state
            .audio
            .sample_playback_session
            .as_mut()
            .expect("sample playback session")
            .state = SamplePlaybackSessionState::RuntimePending;

        state.apply_runtime_playback_progress(wavecrate::audio::PlaybackRuntimeProgress {
            active: true,
            elapsed: Some(Duration::from_millis(90)),
            looping: false,
            progress: Some(0.7),
            error: None,
        });

        assert_eq!(
            state.waveform.current.playhead_ratio(),
            Some(0.1),
            "a pending restart should keep the visual cursor at the requested start"
        );
        assert!(
            !state.audio.playback_progress.active,
            "stale runtime progress should not become the active visual clock while the new start is pending"
        );
        assert_eq!(state.audio.playback_visual_progress, None);

        state.audio.playback_progress = wavecrate::audio::PlaybackRuntimeProgress {
            active: true,
            elapsed: Some(Duration::from_millis(90)),
            looping: false,
            progress: Some(0.7),
            error: None,
        };
        state.refresh_runtime_playback_progress();

        assert_eq!(
            state.waveform.current.playhead_ratio(),
            Some(0.1),
            "frame refresh should also ignore progress snapshots until the runtime confirms the new start"
        );
    }
}
