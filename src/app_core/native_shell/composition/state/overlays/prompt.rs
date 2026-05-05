//! Confirm-prompt rendering for the native shell.

use super::*;

/// Render the confirm prompt overlay when it is visible.
pub(in crate::gui::native_shell::state) fn render_confirm_prompt(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    if !model.confirm_prompt.visible {
        return;
    }
    let sizing = style.sizing;
    let confirm_enabled = !prompt_has_validation_error(model);
    let has_target_label = model.confirm_prompt.target_label.is_some();
    let has_input = model.confirm_prompt.input_value.is_some();
    let prompt_visuals = compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        sizing,
        has_input,
        has_target_label,
    );
    let prompt_sections = prompt_visuals.sections;
    let prompt_text_layout = compute_prompt_overlay_text_layout(
        prompt_sections,
        sizing,
        has_target_label,
        model.confirm_prompt.input_error.is_some(),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: prompt_visuals.scrim,
            color: Rgba8 {
                r: style.bg_primary.r,
                g: style.bg_primary.g,
                b: style.bg_primary.b,
                a: style.scrim_modal_alpha,
            },
        }),
    );
    let dialog = prompt_sections.dialog;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: dialog,
            color: style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        dialog,
        style.accent_warning,
        sizing.border_width,
    );

    emit_text(
        text_runs,
        TextRun {
            text: model.confirm_prompt.title.clone(),
            position: prompt_text_layout.title.min,
            font_size: sizing.font_title,
            color: style.text_primary,
            max_width: Some(prompt_text_layout.title.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: model.confirm_prompt.message.clone(),
            position: prompt_text_layout.message.min,
            font_size: sizing.font_meta,
            color: style.text_muted,
            max_width: Some(prompt_text_layout.message.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    if let (Some(target), Some(target_rect)) = (
        model.confirm_prompt.target_label.as_deref(),
        prompt_text_layout.target,
    ) {
        emit_text(
            text_runs,
            TextRun {
                text: target.to_string(),
                position: target_rect.min,
                font_size: sizing.font_meta,
                color: style.accent_copper,
                max_width: Some(target_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
    if let Some(input_rect) = prompt_sections.input {
        render_prompt_input(
            primitives,
            text_runs,
            style,
            model,
            input_rect,
            prompt_text_layout.input_text,
            prompt_text_layout.input_error,
        );
    }
    let confirm_button = prompt_sections.confirm_button;
    let cancel_button = prompt_sections.cancel_button;
    for (index, (rect, label, color)) in [
        (
            confirm_button,
            if model.confirm_prompt.confirm_label.is_empty() {
                "Confirm"
            } else {
                model.confirm_prompt.confirm_label.as_str()
            },
            style.accent_mint,
        ),
        (
            cancel_button,
            if model.confirm_prompt.cancel_label.is_empty() {
                "Cancel"
            } else {
                model.confirm_prompt.cancel_label.as_str()
            },
            style.text_muted,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let enabled = if index == 0 { confirm_enabled } else { true };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: if enabled {
                    style.surface_overlay
                } else {
                    style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            rect,
            if enabled { color } else { style.border },
            sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: label.to_string(),
                position: if index == 0 {
                    prompt_text_layout.confirm_label.min
                } else {
                    prompt_text_layout.cancel_label.min
                },
                font_size: sizing.font_meta,
                color: if !enabled {
                    style.text_muted
                } else if index == 0 {
                    style.text_primary
                } else {
                    style.text_muted
                },
                max_width: Some(if index == 0 {
                    prompt_text_layout.confirm_label.width().max(12.0)
                } else {
                    prompt_text_layout.cancel_label.width().max(12.0)
                }),
                align: TextAlign::Center,
            },
        );
    }
}

fn render_prompt_input(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    model: &AppModel,
    input_rect: Rect,
    input_text_rect: Option<Rect>,
    input_error_rect: Option<Rect>,
) {
    let sizing = style.sizing;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: input_rect,
            color: style.surface_base,
        }),
    );
    push_border(
        primitives,
        input_rect,
        if model.confirm_prompt.input_error.is_some() {
            style.accent_warning
        } else {
            style.accent_copper
        },
        sizing.border_width,
    );
    let input_text = model
        .confirm_prompt
        .input_value
        .as_deref()
        .unwrap_or_default();
    let (text, color) = if input_text.is_empty() {
        (
            model
                .confirm_prompt
                .input_placeholder
                .as_deref()
                .unwrap_or("Type here…"),
            style.text_muted,
        )
    } else {
        (input_text, style.text_primary)
    };
    let resolved_input_text_rect =
        input_text_rect.unwrap_or(Rect::from_min_max(input_rect.min, input_rect.min));
    let input_text_width = input_text_rect
        .map(|line_rect| line_rect.width().max(24.0))
        .unwrap_or_else(|| (input_rect.width() - (sizing.text_inset_x * 2.0)).max(24.0));
    emit_text(
        text_runs,
        TextRun {
            text: text.to_string(),
            position: resolved_input_text_rect.min,
            font_size: sizing.font_meta,
            color,
            max_width: Some(input_text_width),
            align: TextAlign::Left,
        },
    );
    if let (Some(error), Some(error_rect)) = (
        model.confirm_prompt.input_error.as_deref(),
        input_error_rect,
    ) {
        emit_text(
            text_runs,
            TextRun {
                text: error.to_string(),
                position: error_rect.min,
                font_size: sizing.font_meta,
                color: style.accent_warning,
                max_width: Some(error_rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
}
