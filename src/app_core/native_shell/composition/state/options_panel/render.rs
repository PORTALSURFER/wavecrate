//! Primitive/text rendering for the native-shell options panel.

use super::geometry::options_panel_layout;
use super::style::{
    inset_rect, status_options_button_border, status_options_button_fill,
    status_options_button_label_color,
};
use super::*;

pub(super) fn render_status_options_button(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    _chip_label: &str,
    error: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    let fill = status_options_button_fill(style, error, hovered, flashed, motion_wave);
    let border = status_options_button_border(style, error, hovered, flashed, motion_wave);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button_rect,
            color: fill,
        }),
    );
    push_border(primitives, button_rect, border, sizing.border_width);
}

pub(super) fn render_options_panel(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    let Some(panel) = options_panel_layout(layout, style, model) else {
        return;
    };
    let sizing = style.sizing;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: panel.panel_rect,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        panel.panel_rect,
        blend_color(style.border_emphasis, style.highlight_orange, 0.42),
        sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: panel.title.clone(),
            position: panel.title_rect.min,
            font_size: sizing.font_title,
            color: style.text_primary,
            max_width: Some(panel.title_rect.width().max(36.0)),
            align: TextAlign::Left,
        },
    );
    if let (Some(detail_rect), Some(detail)) = (
        panel.detail_rect,
        model.paired_device_panel().detail_label(),
    ) {
        emit_text(
            text_runs,
            TextRun {
                text: detail.to_string(),
                position: detail_rect.min,
                font_size: sizing.font_meta,
                color: blend_color(style.accent_trash, style.text_primary, 0.25),
                max_width: Some(detail_rect.width().max(36.0)),
                align: TextAlign::Left,
            },
        );
    }
    for button in &panel.buttons {
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.active {
                    translucent_overlay_color(style.surface_base, style.accent_mint, 0.22)
                } else {
                    style.surface_base
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.active {
                blend_color(style.accent_mint, style.text_primary, 0.28)
            } else {
                blend_color(style.border_emphasis, style.text_primary, 0.18)
            },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.text.clone(),
                position: label_rect.min,
                font_size: sizing.font_meta,
                color: if button.text.starts_with("YOLO Edits: On") {
                    style.accent_warning
                } else {
                    style.text_primary
                },
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Left,
            },
        );
    }
}

pub(super) fn render_status_options_button_label(
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    button_rect: Rect,
    chip_label: &str,
    error: bool,
    hovered: bool,
    flashed: bool,
    motion_wave: f32,
) {
    let label_rect = inset_rect(button_rect, sizing.text_inset_x.max(4.0), 0.0);
    emit_text(
        text_runs,
        TextRun {
            text: chip_label.to_string(),
            position: label_rect.min,
            font_size: sizing.font_meta,
            color: status_options_button_label_color(style, error, hovered, flashed, motion_wave),
            max_width: Some(label_rect.width().max(24.0)),
            align: TextAlign::Center,
        },
    );
}
