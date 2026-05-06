use super::*;
use crate::compat_app_contract::{BrowserPillModel, BrowserPillState};

/// Render the left-sidebar tag editor panel.
pub(super) fn render_sidebar_tags(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let rect = sidebar_workspace_sections(ctx.layout, ctx.style).tags;
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }
    render_section_panel(ctx, primitives, rect);
    render_section_title(ctx, text_runs, rect, "TAGS");

    let input = sidebar_tag_input_rect(rect, ctx.sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: input,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        input,
        ctx.style.border_emphasis,
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: if ctx.model.browser.pill_editor().input_value.is_empty() {
                String::from("Add tag")
            } else {
                ctx.model.browser.pill_editor().input_value.clone()
            },
            position: sidebar_tag_input_text_rect(input, ctx.sizing).min,
            font_size: ctx.sizing.font_meta,
            color: if ctx.model.browser.pill_editor().input_value.is_empty() {
                ctx.style.text_muted
            } else {
                ctx.style.text_primary
            },
            max_width: Some(
                sidebar_tag_input_text_rect(input, ctx.sizing)
                    .width()
                    .max(24.0),
            ),
            align: TextAlign::Left,
        },
    );

    for (pill, pill_rect) in sidebar_tag_pill_rects(rect, ctx.sizing, ctx.model) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: pill_rect,
                color: tag_pill_fill(ctx, pill.state),
            }),
        );
        push_border(
            primitives,
            pill_rect,
            tag_pill_border(ctx, pill.state),
            ctx.sizing.border_width,
        );
        let text_rect = inset_rect(pill_rect, ctx.sizing.text_inset_x, ctx.sizing.text_inset_y);
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    &pill.label,
                    text_rect.width().max(18.0),
                    ctx.sizing.font_meta,
                ),
                position: text_rect.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_primary,
                max_width: Some(text_rect.width().max(18.0)),
                align: TextAlign::Left,
            },
        );
        let close_rect = Rect::from_min_max(
            Point::new(
                (pill_rect.max.x - ctx.sizing.font_meta - 4.0).max(pill_rect.min.x),
                pill_rect.min.y,
            ),
            pill_rect.max,
        );
        emit_text(
            text_runs,
            TextRun {
                text: String::from("x"),
                position: inset_rect(close_rect, 2.0, ctx.sizing.text_inset_y).min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(close_rect.width().max(8.0)),
                align: TextAlign::Center,
            },
        );
    }
}

/// Return the sidebar tag input hit/render rectangle.
pub(in crate::gui::native_shell::state) fn sidebar_tag_input_rect(
    rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let height = sizing.browser_row_height.max(18.0);
    Rect::from_min_max(
        Point::new(
            rect.min.x + pad,
            (rect.max.y - pad - height).max(rect.min.y + pad),
        ),
        Point::new(rect.max.x - pad, rect.max.y - pad),
    )
}

/// Return the inset text box inside the sidebar tag input.
pub(in crate::gui::native_shell::state) fn sidebar_tag_input_text_rect(
    input: Rect,
    sizing: SizingTokens,
) -> Rect {
    inset_rect(input, sizing.text_inset_x, sizing.text_inset_y)
}

/// Return visible sidebar tag pill rectangles paired with their pill models.
pub(in crate::gui::native_shell::state) fn sidebar_tag_pill_rects(
    rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Vec<(&BrowserPillModel, Rect)> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 3.0;
    let title_height = sizing.font_meta + sizing.text_inset_y + 2.0;
    let input = sidebar_tag_input_rect(rect, sizing);
    let row_height = sizing.browser_row_height.max(18.0);
    let col_width = ((rect.width() - pad * 2.0 - gap) * 0.5).max(36.0);
    let mut out = Vec::new();
    let mut pills: Vec<_> = model
        .browser
        .pill_editor()
        .option_pills
        .iter()
        .filter(|pill| !matches!(pill.state, BrowserPillState::Off))
        .collect();
    if pills.is_empty() {
        pills.extend(model.browser.pill_editor().option_pills.iter().take(4));
    }
    if let Some(create) = model.browser.pill_editor().create_pill.as_ref() {
        pills.push(create);
    }
    for (index, pill) in pills.into_iter().take(4).enumerate() {
        let col = index % 2;
        let row = index / 2;
        let min_x = rect.min.x + pad + (col_width + gap) * col as f32;
        let min_y = rect.min.y + pad + title_height + (row_height + gap) * row as f32;
        let pill_rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new(
                (min_x + col_width).min(rect.max.x - pad),
                min_y + row_height,
            ),
        );
        if pill_rect.max.y <= input.min.y - gap {
            out.push((pill, pill_rect));
        }
    }
    out
}

/// Render the shared panel background for sidebar subsections.
fn render_section_panel(ctx: &StaticFrameCtx<'_>, primitives: &mut impl PrimitiveSink, rect: Rect) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: blend_color(ctx.style.bg_secondary, ctx.style.surface_base, 0.42),
        }),
    );
    push_border(primitives, rect, ctx.style.border, ctx.sizing.border_width);
}

/// Render an uppercase sidebar subsection title.
fn render_section_title(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    rect: Rect,
    title: &str,
) {
    let title_rect = inset_rect(
        rect,
        ctx.sizing.panel_inset.max(5.0),
        ctx.sizing.text_inset_y,
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from(title),
            position: title_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(title_rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
}

/// Return the fill color for a tag pill state.
fn tag_pill_fill(ctx: &StaticFrameCtx<'_>, state: BrowserPillState) -> Rgba8 {
    match state {
        BrowserPillState::On => blend_color(ctx.style.accent_mint, ctx.style.surface_overlay, 0.62),
        BrowserPillState::Mixed => {
            blend_color(ctx.style.highlight_orange, ctx.style.surface_overlay, 0.45)
        }
        BrowserPillState::Off => ctx.style.surface_overlay,
    }
}

/// Return the border color for a tag pill state.
fn tag_pill_border(ctx: &StaticFrameCtx<'_>, state: BrowserPillState) -> Rgba8 {
    match state {
        BrowserPillState::On => ctx.style.accent_mint,
        BrowserPillState::Mixed => ctx.style.highlight_orange,
        BrowserPillState::Off => ctx.style.border_emphasis,
    }
}

/// Inset a rectangle without inverting its bounds.
fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(
            (rect.min.x + x).min(rect.max.x),
            (rect.min.y + y).min(rect.max.y),
        ),
        Point::new(
            (rect.max.x - x).max(rect.min.x),
            (rect.max.y - y).max(rect.min.y),
        ),
    )
}
