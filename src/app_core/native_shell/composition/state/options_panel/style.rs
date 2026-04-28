//! Style helpers for the native-shell audio chip and options-panel surface.

use super::*;

pub(super) fn status_options_button_fill(
    style: &StyleTokens,
    error: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let (idle, hover) = if error {
        (
            translucent_overlay_color(style.surface_overlay, style.accent_trash, 0.2),
            translucent_overlay_color(style.surface_overlay, style.accent_trash, 0.34),
        )
    } else {
        (
            style.surface_overlay,
            translucent_overlay_color(
                style.surface_overlay,
                style.highlight_orange_soft,
                0.2 + (motion_wave * 0.04),
            ),
        )
    };
    let flash = blend_color(hover, style.text_primary, 0.18);
    if flashed {
        flash
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(super) fn status_options_button_border(
    style: &StyleTokens,
    error: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let (idle, hover) = if error {
        (
            blend_color(style.border_emphasis, style.accent_trash, 0.42),
            blend_color(
                style.text_primary,
                style.accent_trash,
                0.6 + (motion_wave * 0.06),
            ),
        )
    } else {
        (
            style.border,
            blend_color(
                style.border_emphasis,
                style.highlight_orange,
                0.42 + (motion_wave * 0.06),
            ),
        )
    };
    let flash = blend_color(hover, style.text_primary, 0.18);
    if flashed {
        flash
    } else if hovered {
        hover
    } else {
        idle
    }
}

pub(super) fn status_options_button_label_color(
    style: &StyleTokens,
    error: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) -> Rgba8 {
    let idle = if error {
        blend_color(style.text_primary, style.accent_trash, 0.42)
    } else {
        style.text_muted
    };
    let hover = if error {
        blend_color(
            style.text_primary,
            style.accent_trash,
            0.7 + (motion_wave * 0.08),
        )
    } else {
        blend_color(
            style.text_primary,
            style.highlight_orange,
            0.5 + (motion_wave * 0.08),
        )
    };
    if flashed || hovered { hover } else { idle }
}

pub(super) fn inset_rect(rect: Rect, inset_x: f32, inset_y: f32) -> Rect {
    let min_x = (rect.min.x + inset_x).min(rect.max.x);
    let max_x = (rect.max.x - inset_x).max(min_x);
    let min_y = (rect.min.y + inset_y).min(rect.max.y);
    let max_y = (rect.max.y - inset_y).max(min_y);
    Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y))
}
