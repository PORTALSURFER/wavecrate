use super::*;

/// Horizontal inset used by the waveform scrollbar track.
const WAVEFORM_SCROLLBAR_INSET_X: f32 = 10.0;
/// Bottom inset used by the waveform scrollbar track.
const WAVEFORM_SCROLLBAR_INSET_BOTTOM: f32 = 3.0;
/// Track height used by the waveform scrollbar.
const WAVEFORM_SCROLLBAR_TRACK_HEIGHT: f32 = 6.0;
/// Minimum thumb width for the waveform scrollbar.
const WAVEFORM_SCROLLBAR_MIN_THUMB_WIDTH: f32 = 28.0;

/// Visual geometry for the waveform viewport scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaveformScrollbarLayout {
    /// Full horizontal track lane.
    pub track: Rect,
    /// Draggable thumb that reflects the current waveform view window.
    pub thumb: Rect,
}

/// Emit the horizontal waveform scrollbar that mirrors current viewport bounds.
pub(super) fn emit_waveform_scrollbar(
    primitives: &mut impl PrimitiveSink,
    waveform_scrollbar_lane: Rect,
    style: &StyleTokens,
    model: &NativeMotionModel,
) {
    let Some(scrollbar) = waveform_scrollbar_layout(
        waveform_scrollbar_lane,
        model.waveform_view_start_micros,
        model.waveform_view_end_micros,
    ) else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: scrollbar.track,
            color: blend_color(style.border, style.bg_secondary, 0.22),
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: scrollbar.thumb,
            color: blend_color(style.text_muted, style.text_primary, 0.32),
        }),
    );
}

/// Compute visual scrollbar geometry for the waveform viewport.
pub(crate) fn waveform_scrollbar_layout(
    waveform_scrollbar_lane: Rect,
    view_start_micros: u32,
    view_end_micros: u32,
) -> Option<WaveformScrollbarLayout> {
    if waveform_scrollbar_lane.width() <= 1.0 || waveform_scrollbar_lane.height() <= 1.0 {
        return None;
    }
    let track_min_x = waveform_scrollbar_lane.min.x + WAVEFORM_SCROLLBAR_INSET_X;
    let track_max_x = waveform_scrollbar_lane.max.x - WAVEFORM_SCROLLBAR_INSET_X;
    let track_max_y = waveform_scrollbar_lane.max.y - WAVEFORM_SCROLLBAR_INSET_BOTTOM;
    let track_min_y = (track_max_y - WAVEFORM_SCROLLBAR_TRACK_HEIGHT)
        .max(waveform_scrollbar_lane.min.y)
        .round();
    let track = Rect::from_min_max(
        Point::new(track_min_x.round(), track_min_y),
        Point::new(track_max_x.round(), track_max_y.round()),
    );
    if track.width() <= 1.0 || track.height() <= 1.0 {
        return None;
    }

    let clamped_start = view_start_micros.min(1_000_000);
    let clamped_end = view_end_micros
        .min(1_000_000)
        .max(clamped_start.saturating_add(1));
    let span = clamped_end.saturating_sub(clamped_start).max(1);
    let min_thumb_width = WAVEFORM_SCROLLBAR_MIN_THUMB_WIDTH.min(track.width());
    let thumb_width = (track.width() * (span as f32 / 1_000_000.0))
        .round()
        .clamp(min_thumb_width, track.width());
    let travel = (track.width() - thumb_width).max(0.0);
    let max_view_start = 1_000_000u32.saturating_sub(span);
    let start_ratio = if max_view_start == 0 {
        0.0
    } else {
        clamped_start.min(max_view_start) as f32 / max_view_start as f32
    };
    let thumb_min_x = (track.min.x + (travel * start_ratio)).round();
    let thumb_max_x = (thumb_min_x + thumb_width).min(track.max.x);
    let thumb = Rect::from_min_max(
        Point::new(thumb_min_x, track.min.y),
        Point::new(thumb_max_x.max(thumb_min_x + 1.0), track.max.y),
    );
    Some(WaveformScrollbarLayout { track, thumb })
}

/// Resolve the waveform viewport center for a dragged scrollbar thumb position.
pub(crate) fn waveform_scrollbar_center_for_pointer(
    scrollbar: WaveformScrollbarLayout,
    view_start_micros: u32,
    view_end_micros: u32,
    pointer_x: f32,
    thumb_pointer_offset_x: f32,
) -> Option<u32> {
    let clamped_start = view_start_micros.min(1_000_000);
    let clamped_end = view_end_micros
        .min(1_000_000)
        .max(clamped_start.saturating_add(1));
    let span = clamped_end.saturating_sub(clamped_start).max(1);
    let max_view_start = 1_000_000u32.saturating_sub(span);
    let thumb_width = scrollbar.thumb.width().max(1.0);
    let travel = (scrollbar.track.width() - thumb_width).max(0.0);
    if travel <= f32::EPSILON || max_view_start == 0 {
        return Some(500_000);
    }
    let thumb_min_x = (pointer_x - thumb_pointer_offset_x)
        .clamp(scrollbar.track.min.x, scrollbar.track.max.x - thumb_width);
    let start_ratio = ((thumb_min_x - scrollbar.track.min.x) / travel).clamp(0.0, 1.0);
    let view_start = ((start_ratio * max_view_start as f32).round() as u32).min(max_view_start);
    Some((view_start + (span / 2)).min(1_000_000))
}
