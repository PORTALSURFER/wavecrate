use super::{
    GuiAppState, Instant, PLAYBACK_START_ACTIVE_SOURCE_GRACE, WAVEFORM_SIGNAL_WIDGET_ID,
    WAVEFORM_WIDGET_ID, emit_gui_action,
};
use radiant::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintFillRect, PaintPrimitive, TransientOverlayContext},
};

const PLAYBACK_CURSOR_COLOR: Rgba8 = Rgba8 {
    r: 71,
    g: 220,
    b: 255,
    a: 245,
};
const PLAYBACK_CURSOR_WIDTH: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) struct FrameRepaintScopeSnapshot {
    playing: bool,
    play_selection_flash_active: bool,
    folder_progress_active: bool,
    normalization_progress_active: bool,
    waveform_loading_active: bool,
    sample_loading: bool,
    audio_opening: bool,
    startup_auto_load_pending: bool,
    pending_playback_start: bool,
}

impl GuiAppState {
    pub(in crate::gui_app) fn frame_repaint_scope_before_update(
        &self,
    ) -> FrameRepaintScopeSnapshot {
        FrameRepaintScopeSnapshot::from_state(self)
    }

    pub(in crate::gui_app) fn frame_can_use_paint_only(
        &self,
        before: FrameRepaintScopeSnapshot,
    ) -> bool {
        let after = FrameRepaintScopeSnapshot::from_state(self);
        before.playing
            && after.playing
            && !before.requires_surface_frame()
            && !after.requires_surface_frame()
    }

    pub(in crate::gui_app) fn sync_edit_fade_audio_state(&mut self) {
        if let Some(player) = self.audio_player.as_ref() {
            player.set_edit_fade_state(self.waveform.edit_selection());
        }
    }

    pub(in crate::gui_app) fn refresh_playback_progress(&mut self) {
        let Some(player) = self.audio_player.as_mut() else {
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
        let should_be_looping = self.loop_playback && self.waveform.is_playing();
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
                self.waveform.set_playhead_ratio(progress);
            }
        } else if self.waveform.is_playing() {
            self.finish_playback_progress();
        }
    }

    pub(in crate::gui_app) fn paint_playback_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(progress) = self.current_audio_progress_ratio() else {
            return;
        };
        let Some(visible_ratio) = self.waveform.visible_ratio_for_absolute(progress) else {
            return;
        };
        let Some(bounds) = context
            .plan
            .first_widget_rect(WAVEFORM_SIGNAL_WIDGET_ID)
            .or_else(|| context.plan.first_widget_rect(WAVEFORM_WIDGET_ID))
        else {
            return;
        };
        push_playback_cursor(primitives, bounds, visible_ratio);
    }

    fn stop_playback_after_progress_error(&mut self, error: String) {
        let started_at = Instant::now();
        self.waveform.stop_playback();
        self.sample_status = format!("Playback stopped: {error}");
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
            self.loop_playback = false;
            self.waveform.stop_playback();
            self.current_playback_span = None;
            self.sample_status = format!("Loop playback stopped: {err}");
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
        self.waveform.stop_playback();
        self.current_playback_span = None;
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
    fn from_state(state: &GuiAppState) -> Self {
        Self {
            playing: state.waveform.is_playing(),
            play_selection_flash_active: state.waveform.play_selection_flash_active(),
            folder_progress_active: state.folder_progress.is_some(),
            normalization_progress_active: state.normalization_progress.is_some(),
            waveform_loading_active: state.waveform_loading_label.is_some(),
            sample_loading: state.sample_load_task.active().is_some(),
            audio_opening: state.audio_open_task.active().is_some(),
            startup_auto_load_pending: state.startup_auto_load_pending,
            pending_playback_start: state.pending_playback_start.is_some(),
        }
    }

    fn requires_surface_frame(self) -> bool {
        self.play_selection_flash_active
            || self.folder_progress_active
            || self.normalization_progress_active
            || self.waveform_loading_active
            || self.sample_loading
            || self.audio_opening
            || self.startup_auto_load_pending
            || self.pending_playback_start
    }
}

fn push_playback_cursor(primitives: &mut Vec<PaintPrimitive>, bounds: Rect, ratio: f32) {
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return;
    }
    let cursor_width = PLAYBACK_CURSOR_WIDTH
        .ceil()
        .max(2.0)
        .min(bounds.width().max(1.0));
    let x = (bounds.min.x + bounds.width() * ratio.clamp(0.0, 1.0))
        .round()
        .clamp(bounds.min.x, bounds.max.x);
    let left = (x - cursor_width * 0.5).clamp(
        bounds.min.x,
        (bounds.max.x - cursor_width).max(bounds.min.x),
    );
    let right = (left + cursor_width).min(bounds.max.x);
    if right <= left {
        return;
    }
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: WAVEFORM_WIDGET_ID,
        rect: Rect::from_min_max(
            Point::new(left, bounds.min.y),
            Point::new(right, bounds.max.y),
        ),
        color: PLAYBACK_CURSOR_COLOR,
    }));
}
