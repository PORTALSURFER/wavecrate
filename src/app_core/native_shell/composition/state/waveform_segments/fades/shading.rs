use super::super::*;
use super::curve::{FadeCurveDirection, FadeCurveTrace, emit_edit_fade_curve_trace};
use super::model::{EditFadeOverlayGeometry, EditFadePositions};

pub(super) fn emit_fade_shading(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    geometry: EditFadeOverlayGeometry,
    positions: EditFadePositions,
    accent_blue: Rgba8,
) {
    emit_fade_in_shading(primitives, style, geometry, positions, accent_blue);
    emit_fade_out_shading(primitives, style, geometry, positions, accent_blue);
}

fn emit_fade_in_shading(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    geometry: EditFadeOverlayGeometry,
    positions: EditFadePositions,
    accent_blue: Rgba8,
) {
    let selection = geometry.edit_selection_rect;
    if positions.fade_in_x > selection.min.x {
        emit_fade_rect(
            primitives,
            style,
            FadeRect::new(selection.min.x, positions.fade_in_x, selection, 0.22),
            accent_blue,
        );
        emit_edit_fade_curve_trace(
            primitives,
            FadeCurveTrace {
                waveform_plot: geometry.waveform_plot,
                selection,
                start_x: positions.fade_in_mute_x,
                end_x: positions.fade_in_x,
                curve_milli: positions.fade_in_curve_milli,
                direction: FadeCurveDirection::In,
            },
            accent_blue,
            style,
        );
    }
    if positions.fade_in_mute_x < selection.min.x {
        emit_fade_rect(
            primitives,
            style,
            FadeRect::new(positions.fade_in_mute_x, selection.min.x, selection, 0.16),
            accent_blue,
        );
    }
}

fn emit_fade_out_shading(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    geometry: EditFadeOverlayGeometry,
    positions: EditFadePositions,
    accent_blue: Rgba8,
) {
    let selection = geometry.edit_selection_rect;
    if positions.fade_out_x < selection.max.x {
        emit_fade_rect(
            primitives,
            style,
            FadeRect::new(positions.fade_out_x, selection.max.x, selection, 0.22),
            accent_blue,
        );
        emit_edit_fade_curve_trace(
            primitives,
            FadeCurveTrace {
                waveform_plot: geometry.waveform_plot,
                selection,
                start_x: positions.fade_out_x,
                end_x: positions.fade_out_mute_x,
                curve_milli: positions.fade_out_curve_milli,
                direction: FadeCurveDirection::Out,
            },
            accent_blue,
            style,
        );
    }
    if positions.fade_out_mute_x > selection.max.x {
        emit_fade_rect(
            primitives,
            style,
            FadeRect::new(selection.max.x, positions.fade_out_mute_x, selection, 0.16),
            accent_blue,
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct FadeRect {
    left: f32,
    right: f32,
    selection: Rect,
    alpha: f32,
}

impl FadeRect {
    fn new(left: f32, right: f32, selection: Rect, alpha: f32) -> Self {
        Self {
            left,
            right,
            selection,
            alpha,
        }
    }
}

fn emit_fade_rect(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    fade_rect: FadeRect,
    accent_blue: Rgba8,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                Point::new(fade_rect.left, fade_rect.selection.min.y),
                Point::new(fade_rect.right, fade_rect.selection.max.y),
            ),
            color: translucent_overlay_color(style.surface_overlay, accent_blue, fade_rect.alpha),
        }),
    );
}
