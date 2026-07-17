use super::super::diagnostics::PlayheadOverlayFrameDiagnostics;
use crate::native_app::{
    app::{NativeAppState, SamplePlaybackSession, SamplePlaybackSessionState},
    waveform::{WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID},
};
use radiant::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintPrimitive, TransientOverlayContext, WidgetPaint},
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

impl NativeAppState {
    pub(in crate::native_app) fn paint_playback_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if self.chrome_overlay_suppresses_waveform_transient_overlay() {
            return;
        }
        let Some(projection) = self.playhead_progress_projection_for_frame(context.animation_time)
        else {
            return;
        };
        let Some(visible_ratio) = self
            .waveform
            .current
            .visible_ratio_for_absolute(projection.ratio)
        else {
            return;
        };
        let Some(bounds) = context
            .plan
            .first_widget_rect_by_priority([WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID])
        else {
            return;
        };
        let Some(cursor_x) = push_playback_cursor(primitives, bounds, visible_ratio) else {
            return;
        };
        self.playhead_frame_diagnostics
            .record_overlay_frame(PlayheadOverlayFrameDiagnostics {
                animation_time: context.animation_time,
                progress_ratio: projection.ratio,
                visible_ratio,
                cursor_x,
                progress_source: projection.source,
            });
    }

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

    pub(in crate::native_app) fn should_paint_app_transient_overlay(&self) -> bool {
        !self.chrome_overlay_suppresses_waveform_transient_overlay()
            && (self.playback_visual_activity_active()
                || self.waveform.load.label.is_some()
                || self.source_processing_activity_overlay_visible()
                || self.active_starmap_audition_file_id().is_some())
    }

    #[cfg(test)]
    pub(in crate::native_app) fn should_paint_waveform_transient_overlay(&self) -> bool {
        !self.chrome_overlay_suppresses_waveform_transient_overlay()
            && (self.playback_visual_activity_active() || self.waveform.load.label.is_some())
    }

    pub(in crate::native_app) fn playback_visual_activity_active(&self) -> bool {
        self.waveform.current.is_playing()
            || self.audio.playback_progress.active
            || self
                .audio
                .sample_playback_session
                .as_ref()
                .is_some_and(sample_playback_session_pending_start)
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

    pub(in crate::native_app) fn active_starmap_audition_file_id(&self) -> Option<&str> {
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
}

fn push_playback_cursor(
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    ratio: f32,
) -> Option<f32> {
    let width = PLAYBACK_CURSOR_WIDTH
        .ceil()
        .clamp(1.0, bounds.width().max(1.0));
    let center_x = bounds.x_for_ratio(ratio.clamp(0.0, 1.0));
    let left =
        (center_x - width * 0.5).clamp(bounds.min.x, (bounds.max.x - width).max(bounds.min.x));
    let right = (left + width).min(bounds.max.x);
    if right <= left {
        return None;
    }
    WidgetPaint::new(primitives, WAVEFORM_WIDGET_ID).push_visible_fill_rect(
        Rect::from_min_max(
            Point::new(left, bounds.min.y),
            Point::new(right, bounds.max.y),
        ),
        PLAYBACK_CURSOR_COLOR,
    );
    Some(center_x)
}

fn sample_playback_session_pending_start(session: &SamplePlaybackSession) -> bool {
    matches!(
        session.state,
        SamplePlaybackSessionState::ResolvingSource | SamplePlaybackSessionState::RuntimePending
    )
}
