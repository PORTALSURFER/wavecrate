//! Edit-selection overlay emission.

use super::*;
use crate::{
    app_core::native_shell::runtime_contract::NormalizedRangeModel,
    gui::{types::Rgba8, visualization::TimelineViewport},
};

pub(super) fn emit_waveform_edit_selection(
    primitives: &mut impl PrimitiveSink,
    input: &WaveformOverlayInput<'_>,
    viewport: &TimelineViewport,
) {
    let edit_preview = input.model.waveform_edit_preview();
    let Some(edit_selection) = edit_preview.selection else {
        return;
    };
    let Some(rect) = edit_selection_rect(input, edit_selection, viewport) else {
        return;
    };

    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: edit_selection_fill(input.style, &input.flashes),
        }),
    );
    push_border(
        primitives,
        rect,
        edit_selection_border(input.style, &input.flashes),
        input.style.sizing.border_width,
    );
    emit_edit_fade_overlays(
        primitives,
        input.style,
        EditFadeOverlayGeometry {
            waveform_plot: input.layout.waveform_plot,
            edit_selection_rect: rect,
            view_start_micros: viewport.start_micros,
            view_end_micros: viewport.end_micros,
        },
        EditFadeSelection {
            range: edit_selection,
            fade_in: EditFadeSide {
                inner: EditFadeTime::new(
                    edit_preview.leading_end_milli,
                    edit_preview.leading_end_micros,
                ),
                outer: EditFadeTime::new(
                    edit_preview.leading_inner_start_milli,
                    edit_preview.leading_inner_start_micros,
                ),
                curve_milli: edit_preview.leading_curve_milli,
            },
            fade_out: EditFadeSide {
                inner: EditFadeTime::new(
                    edit_preview.trailing_start_milli,
                    edit_preview.trailing_start_micros,
                ),
                outer: EditFadeTime::new(
                    edit_preview.trailing_inner_end_milli,
                    edit_preview.trailing_inner_end_micros,
                ),
                curve_milli: edit_preview.trailing_curve_milli,
            },
        },
        input.style.highlight_blue,
    );
    emit_hovered_edit_resize_edge(
        primitives,
        input.style,
        rect,
        input.style.highlight_blue,
        input.hovered_resize_edge,
    );
    emit_selection_shift_handle(primitives, input.style, rect, input.style.highlight_blue);
}

fn edit_selection_rect(
    input: &WaveformOverlayInput<'_>,
    edit_selection: NormalizedRangeModel,
    viewport: &TimelineViewport,
) -> Option<Rect> {
    compute_waveform_annotation_rects_with_nanos(
        input.layout.waveform_plot,
        input.style.sizing.border_width,
        Some(edit_selection),
        None,
        None,
        viewport.start_micros,
        viewport.end_micros,
        viewport.start_nanos,
        viewport.end_nanos,
    )
    .selection
}

fn edit_selection_fill(style: &StyleTokens, flashes: &WaveformOverlayFlashes) -> Rgba8 {
    if flashes.edit_selection_active {
        return translucent_overlay_color(style.surface_overlay, style.highlight_blue, 0.82);
    }

    translucent_overlay_color(style.bg_secondary, style.highlight_blue, 0.5)
}

fn edit_selection_border(style: &StyleTokens, flashes: &WaveformOverlayFlashes) -> Rgba8 {
    if flashes.edit_selection_active {
        return blend_color(style.highlight_blue, style.text_primary, 0.5);
    }

    blend_color(style.highlight_blue, style.text_primary, 0.24)
}
