use super::*;

pub(super) fn push_status_right_motion_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    status_right: &str,
) {
    if status_right.is_empty() {
        return;
    }
    let sizing = style.sizing;
    let background_rect =
        status_motion_overlay_rect(layout.status_right_segment, sizing.border_width);
    if background_rect.width() > 0.0 && background_rect.height() > 0.0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: background_rect,
                color: style.surface_raised,
            }),
        );
    }
    let status_text_rect = status_right_text_rect(layout.status_right_segment, sizing, None);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                status_right,
                status_text_rect.width().max(36.0),
                sizing.font_status,
            ),
            position: status_text_rect.min,
            font_size: sizing.font_status,
            color: style.text_muted,
            max_width: Some(status_text_rect.width().max(36.0)),
            align: TextAlign::Right,
        },
    );
}
