//! Waveform header chrome assembly.

use super::*;

pub(in crate::gui::native_shell::state) fn push_waveform_header_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    toolbar_left: Option<f32>,
) {
    let sizing = style.sizing;
    let content = waveform_header_surface_content(model);
    let surface = resolve_waveform_header_surface_layout(layout.waveform_header, sizing, &content);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: layout.waveform_header,
            color: style.surface_raised,
        }),
    );
    let content_right = toolbar_left
        .unwrap_or(layout.waveform_header.max.x - sizing.text_inset_x)
        .clamp(
            surface.title_text_rect.min.x + 24.0,
            layout.waveform_header.max.x,
        );
    let title_max_width = surface
        .title_text_rect
        .width()
        .min((content_right - surface.title_text_rect.min.x).max(24.0))
        .max(24.0);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(&content.title, title_max_width, sizing.font_header),
            position: surface.title_text_rect.min,
            font_size: sizing.font_header,
            color: style.text_primary,
            max_width: Some(title_max_width),
            align: TextAlign::Left,
        },
    );
    let metadata_max_width = surface
        .metadata_text_rect
        .width()
        .min((content_right - surface.metadata_text_rect.min.x).max(24.0))
        .max(24.0);
    emit_text(
        text_runs,
        TextRun {
            text: content.metadata,
            position: surface.metadata_text_rect.min,
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(metadata_max_width),
            align: TextAlign::Left,
        },
    );
}
