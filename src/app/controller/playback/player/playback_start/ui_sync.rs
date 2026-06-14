use super::*;
use std::time::Instant;

pub(super) fn sync_playback_ui(
    controller: &mut AppController,
    start: f32,
    span_end: f32,
    start_override: Option<f64>,
) {
    const RESUME_POSITION_EPSILON: f32 = 0.0005;

    let previous_playhead_visible = controller.ui.waveform.playhead.visible;
    let previous_playhead_position = controller.ui.waveform.playhead.position;
    let is_resume_at_current_playhead = start_override.is_some()
        && previous_playhead_visible
        && previous_playhead_position.is_finite()
        && (previous_playhead_position - start).abs() <= RESUME_POSITION_EPSILON;
    let is_seek = start_override.is_some() && !is_resume_at_current_playhead;

    controller.ui.waveform.playhead.active_span_end = Some(span_end.clamp(0.0, 1.0));
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = start;
    super::super::super::playhead_trail::start_or_seek_trail(
        &mut controller.ui.waveform.playhead,
        start,
        is_seek,
    );
    if is_seek {
        controller.ui.waveform.playhead.recent_seek = Some(crate::app::state::PlayheadSeek {
            position: start,
            started_at: Instant::now(),
        });
    }
}

/// Refresh the waveform raster before playback overlays animate over it.
///
/// Zoom and selection interactions can leave a queued or stale waveform image
/// behind the current view window until the next explicit redraw. When playback
/// starts, the selection/playhead overlays immediately begin using the current
/// zoom bounds, so refresh the raster first when those retained inputs drift.
pub(super) fn refresh_waveform_image_if_view_stale(controller: &mut AppController) {
    let Some(decoded) = controller.sample_view.waveform.decoded.as_ref() else {
        return;
    };
    let Some(render_meta) = controller.sample_view.waveform.render_meta.as_ref() else {
        controller.refresh_waveform_image();
        return;
    };
    let view = controller.ui.waveform.view.clamp();
    let stale_view = (render_meta.view_start - view.start).abs() > f64::EPSILON
        || (render_meta.view_end - view.end).abs() > f64::EPSILON;
    let stale_layout = render_meta.size != controller.sample_view.waveform.size
        || render_meta.samples_len != decoded.frame_count()
        || render_meta.channel_view != controller.ui.waveform.channel_view
        || render_meta.channels != decoded.channels;
    if stale_view || stale_layout || controller.ui.waveform.image.is_none() {
        controller.refresh_waveform_image();
    }
}
