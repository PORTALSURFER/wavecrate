//! Waveform toolbar and BPM input overlay visuals.

use super::super::*;

pub(in crate::gui::native_shell::state) fn render_waveform_bpm_input_focus_overlay(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    input_rect: Rect,
    motion_wave: f32,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: input_rect,
            color: waveform_bpm_input_focus_fill(style, motion_wave),
        }),
    );
    push_border(
        primitives,
        input_rect,
        waveform_bpm_input_focus_border(style, motion_wave),
        sizing.border_width,
    );
}

pub(in crate::gui::native_shell::state) fn waveform_bpm_input_focus_fill(
    style: &StyleTokens,
    motion_wave: f32,
) -> Rgba8 {
    translucent_overlay_color(
        style.surface_base,
        style.highlight_orange_soft,
        0.24 + (motion_wave * 0.05),
    )
}

pub(in crate::gui::native_shell::state) fn waveform_bpm_input_focus_border(
    style: &StyleTokens,
    motion_wave: f32,
) -> Rgba8 {
    blend_color(
        style.border_emphasis,
        style.highlight_orange,
        0.58 + (motion_wave * 0.08),
    )
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_visual_color(
    style: &StyleTokens,
    base_color: Rgba8,
    enabled: bool,
    active: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    if !enabled {
        return blend_color(style.text_muted, style.bg_tertiary, 0.42);
    }
    let idle_color = blend_color(style.text_muted, style.bg_tertiary, 0.26);
    let active_color = if active {
        blend_color(base_color, style.text_primary, 0.08 + (motion_wave * 0.06))
    } else {
        idle_color
    };
    let hover_color = if hovered {
        let hover_emphasis = if active { 0.28 } else { 0.82 };
        blend_color(
            active_color,
            base_color,
            hover_emphasis + (motion_wave * 0.06),
        )
    } else {
        active_color
    };
    if flashed {
        blend_color(hover_color, style.text_primary, 0.42)
    } else {
        hover_color
    }
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_icon_rect(
    button_rect: Rect,
    sizing: SizingTokens,
    active: bool,
    hovered: bool,
    flashed: bool,
) -> Rect {
    let max_side =
        (button_rect.width().min(button_rect.height()) - (sizing.border_width * 4.0)).max(6.0);
    let emphasis = if flashed {
        2.0
    } else if hovered {
        1.0
    } else if active {
        0.6
    } else {
        0.0
    };
    let icon_side = (max_side + emphasis).clamp(8.0, 18.0);
    let offset_x = (button_rect.width() - icon_side).max(0.0) * 0.5;
    let offset_y = (button_rect.height() - icon_side).max(0.0) * 0.5;
    Rect::from_min_max(
        Point::new(button_rect.min.x + offset_x, button_rect.min.y + offset_y),
        Point::new(
            button_rect.min.x + offset_x + icon_side,
            button_rect.min.y + offset_y + icon_side,
        ),
    )
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_overlay_icon_rect(
    button_rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let base = waveform_toolbar_icon_rect(button_rect, sizing, false, false, false);
    let side = (base.width().min(base.height()) * 0.48).clamp(6.0, 10.0);
    let inset = sizing.border_width.max(1.0);
    Rect::from_min_max(
        Point::new(base.max.x - side - inset, base.min.y + inset),
        Point::new(base.max.x - inset, base.min.y + side + inset),
    )
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_overlay_icon_color(
    style: &StyleTokens,
    icon_color: Rgba8,
) -> Rgba8 {
    blend_color(icon_color, style.text_primary, 0.62)
}
