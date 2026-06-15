use std::{path::Path, time::Instant};

use super::PLAYBACK_START_ACTIVE_SOURCE_GRACE;
use crate::native_app::app::{NativeAppState, emit_gui_action, sample_path_label};
use crate::native_app::waveform::{WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID};
use radiant::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintFillRect, PaintPrimitive, TransientOverlayContext, WidgetPaint},
};
use wavecrate::audio::{PlaybackRuntimeCancellation, PlaybackRuntimeEvent, PlaybackRuntimeStarted};

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct FrameRepaintScopeSnapshot {
    playing: bool,
    play_selection_flash_active: bool,
    folder_progress_active: bool,
    normalization_progress_active: bool,
    waveform_loading_active: bool,
    sample_loading: bool,
    audio_opening: bool,
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
        self.audio.playback_events = Some(events);
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
                self.audio.playback_progress = progress;
            }
        }
    }

    fn finish_runtime_playback_started(&mut self, started: PlaybackRuntimeStarted) {
        let Some(pending) = self.audio.pending_runtime_start.take() else {
            return;
        };
        if pending.id != started.id {
            self.audio.pending_runtime_start = Some(pending);
            return;
        }
        self.audio.output_resolved = Some(started.output);
        self.audio.current_playback_span = Some(pending.span);
        if self.waveform.current.path() == Path::new(&pending.path) {
            self.waveform.current.start_playback(started.playback_start);
        }
        self.ui.status.sample = format!("Playing {}", sample_path_label(&pending.path));
    }

    fn finish_runtime_playback_failed(
        &mut self,
        id: wavecrate::audio::PlaybackRequestId,
        error: String,
    ) {
        let Some(pending) = self.audio.pending_runtime_start.take() else {
            return;
        };
        if pending.id != id {
            self.audio.pending_runtime_start = Some(pending);
            return;
        }
        self.audio.early_sample_playback_path = None;
        self.audio.current_playback_span = None;
        self.waveform.current.stop_playback();
        self.ui.status.sample = format!(
            "Loaded {} | playback unavailable: {error}",
            sample_path_label(&pending.path)
        );
    }

    fn finish_runtime_playback_cancelled(
        &mut self,
        id: wavecrate::audio::PlaybackRequestId,
        reason: PlaybackRuntimeCancellation,
    ) {
        let Some(pending) = self.audio.pending_runtime_start.take() else {
            return;
        };
        if pending.id != id {
            self.audio.pending_runtime_start = Some(pending);
            return;
        }
        if reason != PlaybackRuntimeCancellation::Superseded {
            self.audio.early_sample_playback_path = None;
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

        let active = self.audio.playback_progress.active;
        let elapsed = self.audio.playback_progress.elapsed;
        let player_looping = self.audio.playback_progress.looping;
        let progress = self.audio.playback_progress.progress;
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

    pub(in crate::native_app) fn paint_playback_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(progress) = self.current_audio_progress_ratio() else {
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

    pub(in crate::native_app) fn paint_waveform_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_loading_overlay(context, primitives);
        self.paint_playback_overlay(context, primitives);
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
        push_fill(
            primitives,
            WAVEFORM_WIDGET_ID,
            bounds,
            LOADING_BACKGROUND_COLOR,
        );
        let progress = self.waveform.load.progress.clamp(0.0, 1.0);
        if progress <= 0.0 {
            return;
        }
        let progress_bounds = Rect::from_min_max(
            bounds.min,
            Point::new(bounds.min.x + bounds.width() * progress, bounds.max.y),
        );
        push_fill(
            primitives,
            WAVEFORM_WIDGET_ID,
            progress_bounds,
            LOADING_PROGRESS_COLOR,
        );
    }

    fn stop_playback_after_progress_error(&mut self, error: String) {
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

impl FrameRepaintScopeSnapshot {
    fn from_state(state: &NativeAppState) -> Self {
        Self {
            playing: state.waveform.current.is_playing(),
            play_selection_flash_active: state.waveform.current.play_selection_flash_active(),
            folder_progress_active: state.library.folder_scan_active(),
            normalization_progress_active: state.background.normalization_progress.is_some(),
            waveform_loading_active: state.waveform.load.label.is_some(),
            sample_loading: state.active_sample_load_task().is_some(),
            audio_opening: state.background.audio_open.active().is_some(),
            startup_source_scan_pending: state.ui.startup.source_scan_pending,
            startup_auto_load_pending: state.ui.startup.auto_load_pending,
            pending_playback_start: state.audio.pending_playback_start.is_some(),
        }
    }

    fn requires_surface_frame(self) -> bool {
        self.play_selection_flash_active
            || self.folder_progress_active
            || self.normalization_progress_active
            || self.sample_loading
            || self.audio_opening
            || self.startup_source_scan_pending
            || self.startup_auto_load_pending
            || self.pending_playback_start
    }

    fn same_transient_frame_state(self, after: Self) -> bool {
        self.playing == after.playing
            && self.play_selection_flash_active == after.play_selection_flash_active
            && self.folder_progress_active == after.folder_progress_active
            && self.normalization_progress_active == after.normalization_progress_active
            && self.waveform_loading_active == after.waveform_loading_active
            && self.sample_loading == after.sample_loading
            && self.audio_opening == after.audio_opening
            && self.startup_source_scan_pending == after.startup_source_scan_pending
            && self.startup_auto_load_pending == after.startup_auto_load_pending
            && self.pending_playback_start == after.pending_playback_start
    }
}

fn push_playback_cursor(primitives: &mut Vec<PaintPrimitive>, bounds: Rect, ratio: f32) {
    WidgetPaint::new(primitives, WAVEFORM_WIDGET_ID).push_horizontal_value_cursor_fill(
        bounds,
        ratio,
        PLAYBACK_CURSOR_WIDTH,
        PLAYBACK_CURSOR_COLOR,
    );
}

fn push_fill(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: radiant::widgets::WidgetId,
    rect: Rect,
    color: Rgba8,
) {
    if !rect.has_finite_positive_area() {
        return;
    }
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}
