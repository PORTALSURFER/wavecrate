use super::*;

/// One retained ghost line for the dynamic playhead trail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlayheadTrailLine {
    /// Normalized x-position in `0.0..=1.0`.
    pub ratio: f32,
    /// Blend amount for the trail sample or connecting span.
    pub alpha: f32,
}

/// Resolve the active playhead marker rectangle, preferring high-precision micros.
pub(crate) fn playhead_marker_rect(
    waveform_plot: Rect,
    border_width: f32,
    model: &NativeMotionModel,
) -> Option<Rect> {
    let transport = model.waveform_transport();
    let viewport = model.waveform_viewport();
    if let Some(playhead_micros) = transport.playhead_micros {
        return marker_rect_for_absolute_ratio(
            waveform_plot,
            border_width,
            f64::from(playhead_micros) / 1_000_000.0,
            viewport.start_micros,
            viewport.end_micros,
            viewport.start_nanos,
            viewport.end_nanos,
        );
    }
    transport.playhead_milli.and_then(|playhead_milli| {
        marker_rect_for_absolute_ratio(
            waveform_plot,
            border_width,
            f64::from(playhead_milli) / 1000.0,
            viewport.start_micros,
            viewport.end_micros,
            viewport.start_nanos,
            viewport.end_nanos,
        )
    })
}

/// Emit retained ghost lines behind the active playhead.
pub(super) fn emit_waveform_playhead_trail(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    border_width: f32,
    trail_lines: &[PlayheadTrailLine],
    view_start_micros: u32,
    view_end_micros: u32,
    view_start_nanos: u32,
    view_end_nanos: u32,
) {
    let mut previous_rect: Option<Rect> = None;
    let mut previous_alpha = 0.0f32;
    for line in trail_lines {
        let Some(rect) = marker_rect_for_absolute_ratio(
            waveform_plot,
            border_width,
            f64::from(line.ratio),
            view_start_micros,
            view_end_micros,
            view_start_nanos,
            view_end_nanos,
        ) else {
            previous_rect = None;
            previous_alpha = 0.0;
            continue;
        };
        let alpha = line.alpha.clamp(0.0, 1.0);
        if let Some(first_rect) = previous_rect {
            emit_waveform_playhead_trail_segment(
                primitives,
                waveform_plot,
                style,
                first_rect,
                previous_alpha,
                rect,
                alpha,
            );
        } else if alpha > 0.0 {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect,
                    color: translucent_overlay_color(
                        style.surface_overlay,
                        style.accent_copper,
                        alpha,
                    ),
                }),
            );
        }
        previous_rect = Some(rect);
        previous_alpha = alpha;
    }
}

fn marker_rect_for_absolute_ratio(
    waveform_plot: Rect,
    border_width: f32,
    absolute_ratio: f64,
    view_start_micros: u32,
    view_end_micros: u32,
    view_start_nanos: u32,
    view_end_nanos: u32,
) -> Option<Rect> {
    let view = waveform_view_window_from_bounds(
        view_start_micros,
        view_end_micros,
        Some(view_start_nanos),
        Some(view_end_nanos),
    );
    let raw_x = waveform_plot_x_for_absolute_ratio(
        waveform_plot,
        absolute_ratio,
        view,
        NormalizedPixelSnap::Nearest,
    );
    marker_rect_for_x(waveform_plot, border_width, raw_x)
}

fn marker_rect_for_x(waveform_plot: Rect, border_width: f32, raw_x: f32) -> Option<Rect> {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return None;
    }
    let marker_width = border_width.max(1.0).min(waveform_plot.width());
    let left = raw_x.clamp(waveform_plot.min.x, waveform_plot.max.x - marker_width);
    let right = (left + marker_width).min(waveform_plot.max.x);
    Some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(right, waveform_plot.max.y),
    ))
}

fn emit_waveform_playhead_trail_segment(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    start_rect: Rect,
    start_alpha: f32,
    end_rect: Rect,
    end_alpha: f32,
) {
    let start_alpha = start_alpha.clamp(0.0, 1.0);
    let end_alpha = end_alpha.clamp(0.0, 1.0);
    if start_alpha <= 0.0 && end_alpha <= 0.0 {
        return;
    }
    let rect = Rect::from_min_max(
        Point::new(start_rect.min.x.min(end_rect.min.x), waveform_plot.min.y),
        Point::new(start_rect.max.x.max(end_rect.max.x), waveform_plot.max.y),
    );
    emit_primitive(
        primitives,
        Primitive::LinearGradient(FillLinearGradient {
            rect,
            start: Point::new(
                start_rect.min.x + (start_rect.width() * 0.5),
                waveform_plot.min.y,
            ),
            end: Point::new(
                end_rect.min.x + (end_rect.width() * 0.5),
                waveform_plot.min.y,
            ),
            start_color: translucent_overlay_color(
                style.surface_overlay,
                style.accent_copper,
                start_alpha,
            ),
            end_color: translucent_overlay_color(
                style.surface_overlay,
                style.accent_copper,
                end_alpha,
            ),
        }),
    );
}
