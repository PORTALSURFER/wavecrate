use super::*;

/// Emit preview overlays for detected silence-split waveform slices.
pub(in crate::gui::native_shell::state) fn emit_waveform_slice_previews(
    primitives: &mut impl PrimitiveSink,
    waveform_plot: Rect,
    style: &StyleTokens,
    model: &NativeMotionModel,
) {
    let viewport = model.waveform_viewport();
    let slices = compute_waveform_slice_preview_rects(
        waveform_plot,
        &model.waveform_slices,
        viewport.start_micros,
        viewport.end_micros,
    );
    for slice in slices {
        let (fill, border) = if slice.duplicate_cleanup_exempted {
            (
                translucent_overlay_color(style.surface_overlay, style.accent_mint, 0.74),
                blend_color(style.accent_mint, style.text_primary, 0.42),
            )
        } else if slice.duplicate_cleanup_candidate {
            if slice.focused {
                (
                    translucent_overlay_color(style.surface_overlay, style.accent_danger, 0.82),
                    blend_color(style.accent_danger, style.text_primary, 0.55),
                )
            } else {
                (
                    translucent_overlay_color(style.surface_overlay, style.accent_danger, 0.62),
                    blend_color(style.accent_danger, style.text_primary, 0.34),
                )
            }
        } else if slice.focused {
            (
                translucent_overlay_color(style.surface_overlay, style.highlight_blue, 0.82),
                blend_color(style.highlight_blue, style.text_primary, 0.55),
            )
        } else if slice.marked_for_export {
            (
                translucent_overlay_color(style.surface_overlay, style.accent_warning, 0.68),
                blend_color(style.accent_warning, style.text_primary, 0.42),
            )
        } else if slice.selected {
            (
                translucent_overlay_color(style.surface_overlay, style.highlight_blue, 0.72),
                blend_color(style.highlight_blue, style.text_primary, 0.36),
            )
        } else {
            (
                translucent_overlay_color(style.bg_secondary, style.highlight_blue, 0.44),
                blend_color(style.highlight_blue, style.text_primary, 0.18),
            )
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: slice.rect,
                color: fill,
            }),
        );
        push_border(primitives, slice.rect, border, style.sizing.border_width);
    }
}
