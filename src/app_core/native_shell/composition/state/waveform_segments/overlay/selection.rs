//! Play-selection overlay emission.

use super::*;
use crate::gui::types::Rgba8;

pub(super) struct WaveformSelectionOverlay<'a> {
    pub(super) style: &'a StyleTokens,
    pub(super) rect: Rect,
    pub(super) flashes: &'a WaveformOverlayFlashes,
    pub(super) repeat_enabled: bool,
    pub(super) hovered_resize_edge: Option<WaveformResizeHoverEdge>,
}

pub(super) fn emit_waveform_selection(
    primitives: &mut impl PrimitiveSink,
    overlay: WaveformSelectionOverlay<'_>,
) {
    let style = overlay.style;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: overlay.rect,
            color: selection_fill(style, overlay.flashes),
        }),
    );
    push_border(
        primitives,
        overlay.rect,
        selection_border(style, overlay.flashes),
        style.sizing.border_width,
    );
    emit_hovered_selection_resize_edge(
        primitives,
        style,
        overlay.rect,
        style.accent_warning,
        overlay.hovered_resize_edge,
    );
    if overlay.repeat_enabled {
        emit_waveform_loop_bar(primitives, style, overlay.rect);
    }
    emit_selection_shift_handle(primitives, style, overlay.rect, style.accent_warning);
    emit_selection_drag_handle(primitives, style, overlay.rect);
}

fn selection_fill(style: &StyleTokens, flashes: &WaveformOverlayFlashes) -> Rgba8 {
    if !flashes.selection_active {
        return translucent_overlay_color(style.bg_secondary, style.accent_warning, 0.52);
    }

    translucent_overlay_color(
        style.surface_overlay,
        selection_flash_accent(style, flashes),
        0.78,
    )
}

fn selection_border(style: &StyleTokens, flashes: &WaveformOverlayFlashes) -> Rgba8 {
    if !flashes.selection_active {
        return blend_color(style.accent_warning, style.text_primary, 0.28);
    }

    blend_color(
        selection_flash_accent(style, flashes),
        style.text_primary,
        0.5,
    )
}

fn selection_flash_accent(style: &StyleTokens, flashes: &WaveformOverlayFlashes) -> Rgba8 {
    match flashes.selection_tone {
        WaveformSelectionFlashTone::Optimistic => style.accent_warning,
        WaveformSelectionFlashTone::Error => style.accent_danger,
    }
}
