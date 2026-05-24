use super::super::*;

pub(super) fn emit_edit_fade_curve_trace(
    primitives: &mut impl PrimitiveSink,
    trace: FadeCurveTrace,
    accent_blue: Rgba8,
    style: &StyleTokens,
) {
    let width = (trace.end_x - trace.start_x).abs();
    let height = trace.selection.height();
    if width <= 1.0 || height <= 1.0 {
        return;
    }

    let curve = (f32::from(trace.curve_milli.min(1000)) / 1000.0).clamp(0.0, 1.0);
    let steps = ((width / 6.0).round() as usize).clamp(6, 28);
    let marker_size = style.sizing.border_width.max(1.0) + 1.0;
    for step in 0..=steps {
        emit_curve_marker(
            primitives,
            trace,
            CurveMarker {
                t: step as f32 / steps as f32,
                curve,
                marker_size,
            },
            accent_blue,
            style,
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct FadeCurveTrace {
    pub waveform_plot: Rect,
    pub selection: Rect,
    pub start_x: f32,
    pub end_x: f32,
    pub curve_milli: u16,
    pub direction: FadeCurveDirection,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum FadeCurveDirection {
    In,
    Out,
}

impl FadeCurveDirection {
    fn y_for(self, selection: Rect, height: f32, eased: f32) -> f32 {
        match self {
            Self::In => selection.max.y - (height * eased),
            Self::Out => selection.min.y + (height * eased),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CurveMarker {
    t: f32,
    curve: f32,
    marker_size: f32,
}

fn emit_curve_marker(
    primitives: &mut impl PrimitiveSink,
    trace: FadeCurveTrace,
    marker: CurveMarker,
    accent_blue: Rgba8,
    style: &StyleTokens,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: curve_marker_rect(trace, marker),
            color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.88),
        }),
    );
}

fn curve_marker_rect(trace: FadeCurveTrace, marker: CurveMarker) -> Rect {
    let eased = fade_curve_sample(marker.t, marker.curve);
    let x = trace.start_x + ((trace.end_x - trace.start_x) * marker.t);
    let y = trace
        .direction
        .y_for(trace.selection, trace.selection.height(), eased);
    let half = marker.marker_size * 0.5;
    Rect::from_min_max(
        Point::new(
            (x - half).clamp(trace.waveform_plot.min.x, trace.waveform_plot.max.x),
            (y - half).clamp(trace.selection.min.y, trace.selection.max.y),
        ),
        Point::new(
            (x + half).clamp(trace.waveform_plot.min.x, trace.waveform_plot.max.x),
            (y + half).clamp(trace.selection.min.y, trace.selection.max.y),
        ),
    )
}

fn fade_curve_sample(t: f32, curve: f32) -> f32 {
    if curve <= 0.0 {
        return t.clamp(0.0, 1.0);
    }
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let smootherstep = t3 * (t * (t * 6.0 - 15.0) + 10.0);
    t * (1.0 - curve) + smootherstep * curve
}
