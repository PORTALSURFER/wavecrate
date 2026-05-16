use super::*;
use crate::gui::range::{
    NormalizedScrollbar, NormalizedScrollbarRequest, normalized_scrollbar_center_for_pointer,
    resolve_normalized_scrollbar,
};

/// Horizontal inset used by the waveform scrollbar track.
const WAVEFORM_SCROLLBAR_INSET_X: f32 = 10.0;
/// Bottom inset used by the waveform scrollbar track.
const WAVEFORM_SCROLLBAR_INSET_BOTTOM: f32 = 5.0;
/// Track height used by the waveform scrollbar.
const WAVEFORM_SCROLLBAR_TRACK_HEIGHT: f32 = 3.0;
/// Minimum thumb width for the waveform scrollbar.
const WAVEFORM_SCROLLBAR_MIN_THUMB_WIDTH: f32 = 28.0;

/// Visual geometry for the waveform viewport scrollbar.
pub(crate) type WaveformScrollbarLayout = NormalizedScrollbar;

/// Emit the horizontal waveform scrollbar that mirrors current viewport bounds.
pub(super) fn emit_waveform_scrollbar(
    primitives: &mut impl PrimitiveSink,
    waveform_scrollbar_lane: Rect,
    style: &StyleTokens,
    model: &NativeMotionModel,
) {
    let viewport = model.waveform_viewport();
    let Some(scrollbar) = waveform_scrollbar_layout(
        waveform_scrollbar_lane,
        viewport.start_micros,
        viewport.end_micros,
    ) else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: scrollbar.track,
            color: blend_color(style.border, style.bg_secondary, 0.12),
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: scrollbar.thumb,
            color: blend_color(style.text_muted, style.text_primary, 0.18),
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

    resolve_normalized_scrollbar(NormalizedScrollbarRequest {
        track,
        start_micros: view_start_micros,
        end_micros: view_end_micros,
        min_thumb_width: WAVEFORM_SCROLLBAR_MIN_THUMB_WIDTH,
    })
}

/// Resolve the waveform viewport center for a dragged scrollbar thumb position.
pub(crate) fn waveform_scrollbar_center_for_pointer(
    scrollbar: WaveformScrollbarLayout,
    view_start_micros: u32,
    view_end_micros: u32,
    pointer_x: f32,
    thumb_pointer_offset_x: f32,
) -> Option<u32> {
    normalized_scrollbar_center_for_pointer(
        scrollbar,
        view_start_micros,
        view_end_micros,
        pointer_x,
        thumb_pointer_offset_x,
    )
}
