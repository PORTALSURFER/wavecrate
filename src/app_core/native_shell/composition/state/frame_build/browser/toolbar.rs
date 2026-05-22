use super::*;
pub(super) fn render_browser_action_buttons(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    buttons: &[ActionButton],
) {
    for button in buttons {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: browser_action_button_fill(ctx, button),
            }),
        );
        push_border(
            primitives,
            button.rect,
            browser_action_button_border(ctx, button),
            ctx.sizing.border_width,
        );
        render_browser_action_button_content(ctx, primitives, text_runs, button);
    }
}

fn render_browser_action_button_content(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    button: &ActionButton,
) {
    let button_color = if button.enabled {
        button.text_color
    } else {
        ctx.style.text_muted
    };
    if let Some(icon) = button.icon {
        let icon_rect = centered_button_icon_rect(button.rect, ctx.sizing);
        let _ = emit_toolbar_svg_icon(primitives, icon, icon_rect, button_color);
        return;
    }
    let label_rect = compute_action_button_text_rect(button.rect, ctx.sizing);
    emit_text(
        text_runs,
        TextRun {
            text: button.label.to_string(),
            position: label_rect.min,
            font_size: ctx.sizing.font_meta,
            color: button_color,
            max_width: Some(label_rect.width().max(12.0)),
            align: TextAlign::Center,
        },
    );
}

fn browser_action_button_fill(ctx: &StaticFrameCtx<'_>, button: &ActionButton) -> Rgba8 {
    if !button.enabled {
        return ctx.style.control_disabled_fill;
    }
    if button.active {
        return blend_color(ctx.style.highlight_cyan, ctx.style.surface_overlay, 0.26);
    }
    ctx.style.surface_overlay
}

fn browser_action_button_border(ctx: &StaticFrameCtx<'_>, button: &ActionButton) -> Rgba8 {
    if !button.enabled {
        return ctx.style.border;
    }
    if button.active {
        return blend_color(ctx.style.highlight_cyan, ctx.style.text_primary, 0.32);
    }
    blend_color(
        ctx.style.border_emphasis,
        ctx.style.text_primary,
        ctx.style.state_hover_soft,
    )
}

pub(super) fn render_browser_column_chips(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    column_chips: &[BrowserColumnChip],
) {
    for chip in column_chips {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: chip.rect,
                color: browser_column_chip_fill(ctx, chip),
            }),
        );
        push_border(
            primitives,
            chip.rect,
            browser_column_chip_border(ctx, chip),
            ctx.sizing.border_width,
        );
        render_browser_column_chip_text(ctx, text_runs, chip);
    }
}

fn render_browser_column_chip_text(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    chip: &BrowserColumnChip,
) {
    let label_rect = compute_action_button_text_rect(chip.rect, ctx.sizing);
    emit_text(
        text_runs,
        TextRun {
            text: format!("{} ({})", chip.label, chip.item_count),
            position: label_rect.min,
            font_size: ctx.sizing.font_meta,
            color: if chip.selected {
                ctx.style.text_primary
            } else {
                ctx.style.text_muted
            },
            max_width: Some(label_rect.width().max(16.0)),
            align: TextAlign::Center,
        },
    );
}

fn browser_column_chip_fill(ctx: &StaticFrameCtx<'_>, chip: &BrowserColumnChip) -> Rgba8 {
    match (chip.selected, chip.column) {
        (true, 0) => blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.50),
        (true, 2) => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.50),
        (true, _) => blend_color(ctx.style.text_primary, ctx.style.bg_secondary, 0.42),
        (false, 0) => blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.34),
        (false, 2) => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.34),
        (false, _) => blend_color(ctx.style.text_muted, ctx.style.bg_secondary, 0.28),
    }
}

fn browser_column_chip_border(ctx: &StaticFrameCtx<'_>, chip: &BrowserColumnChip) -> Rgba8 {
    if chip.selected {
        blend_color(ctx.style.border_emphasis, ctx.style.text_primary, 0.55)
    } else {
        ctx.style.border
    }
}
