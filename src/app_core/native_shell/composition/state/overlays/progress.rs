//! Progress-overlay rendering for the native shell.

use super::*;

/// Render the modal progress overlay when it is visible.
pub(in crate::gui::native_shell::state) fn render_progress_overlay(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.progress_overlay.visible || !model.progress_overlay.modal {
        return;
    }
    let sizing = style.sizing;
    let fraction = if model.progress_overlay.total == 0 {
        0.0
    } else {
        (model.progress_overlay.completed as f32 / model.progress_overlay.total as f32)
            .clamp(0.0, 1.0)
    };
    let progress_visuals = compute_progress_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        sizing,
        model.progress_overlay.modal,
        fraction,
    );
    if let Some(scrim_rect) = progress_visuals.scrim {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrim_rect,
                color: Rgba8 {
                    r: style.bg_primary.r,
                    g: style.bg_primary.g,
                    b: style.bg_primary.b,
                    a: style.scrim_soft_alpha,
                },
            }),
        );
    }
    let overlay_sections = progress_visuals.sections;
    let progress_text_layout = compute_progress_overlay_text_layout(
        overlay_sections,
        sizing,
        model.progress_overlay.detail.is_some(),
        model.progress_overlay.cancelable,
    );
    let rect = overlay_sections.dialog;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: style.surface_overlay,
        }),
    );
    push_border(primitives, rect, style.border, sizing.border_width);

    emit_text(
        text_runs,
        TextRun {
            text: model.progress_overlay.title.clone(),
            position: progress_text_layout.title.min,
            font_size: sizing.font_header,
            color: style.text_primary,
            max_width: Some(progress_text_layout.title.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    if let (Some(detail), Some(detail_rect)) = (
        model.progress_overlay.detail.as_deref(),
        progress_text_layout.detail,
    ) {
        emit_text(
            text_runs,
            TextRun {
                text: detail.to_string(),
                position: detail_rect.min,
                font_size: sizing.font_meta,
                color: style.text_muted,
                max_width: Some(detail_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
    let bar_rect = overlay_sections.progress_bar;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: bar_rect,
            color: style.grid_soft,
        }),
    );
    if let Some(fill_rect) = progress_visuals.progress_fill {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: fill_rect,
                color: style.accent_mint,
            }),
        );
    }
    push_border(primitives, bar_rect, style.border, sizing.border_width);

    emit_text(
        text_runs,
        TextRun {
            text: format!(
                "{} / {}",
                model.progress_overlay.completed, model.progress_overlay.total
            ),
            position: progress_text_layout.counter.min,
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(progress_text_layout.counter.width().max(24.0)),
            align: TextAlign::Right,
        },
    );

    if model.progress_overlay.cancelable {
        let button = overlay_sections.cancel_button;
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button,
                color: if model.progress_overlay.cancel_requested {
                    style.grid_soft
                } else {
                    style.bg_tertiary
                },
            }),
        );
        push_border(
            primitives,
            button,
            if model.progress_overlay.cancel_requested {
                style.border
            } else {
                style.accent_warning
            },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: if model.progress_overlay.cancel_requested {
                    String::from("Cancelling")
                } else {
                    String::from("Cancel")
                },
                position: progress_text_layout.cancel_label.min,
                font_size: sizing.font_meta,
                color: if model.progress_overlay.cancel_requested {
                    style.text_muted
                } else {
                    style.text_primary
                },
                max_width: Some(progress_text_layout.cancel_label.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }
}
