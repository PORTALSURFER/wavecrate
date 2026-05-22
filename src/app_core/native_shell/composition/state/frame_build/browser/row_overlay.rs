use super::row_overlay_layout::{
    BrowserPillEditorLayout, browser_pill_editor_layout, browser_pill_editor_rect,
};
use super::*;
use crate::app_core::native_shell::runtime_contract::{BrowserPillModel, BrowserPillState};

pub(super) fn render_browser_pill_editor_overlay(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let Some(layout) = browser_pill_editor_layout(ctx.layout.browser_rows, ctx.sizing, ctx.model)
    else {
        return;
    };
    let sidebar = ctx.model.browser.pill_editor();
    let panel_rect = browser_pill_editor_rect(ctx.layout.browser_rows, ctx.sizing, ctx.model)
        .unwrap_or(ctx.layout.browser_rows);
    render_editor_panel(ctx, primitives, text_runs, panel_rect, &layout);
    render_sidebar_toggle_button(
        primitives,
        text_runs,
        ctx,
        layout.auto_rename_rect,
        "Auto-rename",
        sidebar.primary_action_enabled,
    );
    render_editor_input(ctx, primitives, text_runs, &layout);
    for (pill, rect) in sidebar
        .exclusive_pills
        .iter()
        .zip(layout.playback_rects.iter())
    {
        render_sidebar_tag_pill(primitives, text_runs, ctx, *rect, pill);
    }
    for (pill, rect) in sidebar
        .option_pills
        .iter()
        .zip(layout.normal_tag_rects.iter())
    {
        render_sidebar_tag_pill(primitives, text_runs, ctx, *rect, pill);
    }
    if let (Some(create), Some(rect)) = (sidebar.create_pill.as_ref(), layout.create_tag_rect) {
        render_sidebar_tag_pill(primitives, text_runs, ctx, rect, create);
    }
}

fn render_editor_panel(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    panel_rect: Rect,
    layout: &BrowserPillEditorLayout,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: panel_rect,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        panel_rect,
        ctx.style.border_emphasis,
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: ctx.model.browser.pill_editor().header_label.clone(),
            position: Point::new(
                layout.input_rect.min.x,
                panel_rect.min.y + ctx.sizing.panel_inset.max(8.0),
            ),
            font_size: ctx.sizing.font_body,
            color: ctx.style.text_primary,
            max_width: Some((panel_rect.width() - ctx.sizing.panel_inset * 2.0).max(24.0)),
            align: TextAlign::Left,
        },
    );
}

fn render_editor_input(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &BrowserPillEditorLayout,
) {
    let sidebar = ctx.model.browser.pill_editor();
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: layout.input_rect,
            color: ctx.style.surface_base,
        }),
    );
    push_border(
        primitives,
        layout.input_rect,
        ctx.style.border,
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: if sidebar.input_value.is_empty() {
                sidebar.input_placeholder.clone()
            } else {
                sidebar.input_value.clone()
            },
            position: layout.input_text_rect.min,
            font_size: ctx.sizing.font_meta,
            color: if sidebar.input_value.is_empty() {
                ctx.style.text_muted
            } else {
                ctx.style.text_primary
            },
            max_width: Some(layout.input_text_rect.width().max(20.0)),
            align: TextAlign::Left,
        },
    );
}

fn render_sidebar_toggle_button(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    rect: Rect,
    label: &str,
    active: bool,
) {
    let (fill, border, text) = active_button_colors(ctx, active);
    emit_primitive(primitives, Primitive::Rect(FillRect { rect, color: fill }));
    push_border(primitives, rect, border, ctx.sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: label.to_string(),
            position: Point::new(
                rect.min.x + ctx.sizing.text_inset_x,
                rect.min.y + ctx.sizing.text_inset_y,
            ),
            font_size: ctx.sizing.font_meta,
            color: text,
            max_width: Some((rect.width() - ctx.sizing.text_inset_x * 2.0).max(10.0)),
            align: TextAlign::Left,
        },
    );
}

fn render_sidebar_tag_pill(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    rect: Rect,
    pill: &BrowserPillModel,
) {
    let (fill, border, text) = pill_colors(ctx, pill.state);
    emit_primitive(primitives, Primitive::Rect(FillRect { rect, color: fill }));
    push_border(primitives, rect, border, ctx.sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: pill.label.clone(),
            position: Point::new(
                rect.min.x + ctx.sizing.text_inset_x,
                rect.min.y + ctx.sizing.text_inset_y,
            ),
            font_size: ctx.sizing.font_meta,
            color: text,
            max_width: Some((rect.width() - ctx.sizing.text_inset_x * 2.0).max(10.0)),
            align: TextAlign::Left,
        },
    );
}

fn active_button_colors(ctx: &StaticFrameCtx<'_>, active: bool) -> (Rgba8, Rgba8, Rgba8) {
    if active {
        (
            blend_color(ctx.style.highlight_cyan, ctx.style.surface_overlay, 0.24),
            blend_color(ctx.style.highlight_cyan, ctx.style.text_primary, 0.32),
            ctx.style.text_primary,
        )
    } else {
        (
            ctx.style.surface_base,
            ctx.style.border,
            ctx.style.text_muted,
        )
    }
}

fn pill_colors(ctx: &StaticFrameCtx<'_>, state: BrowserPillState) -> (Rgba8, Rgba8, Rgba8) {
    match state {
        BrowserPillState::Off => (
            ctx.style.surface_base,
            ctx.style.border,
            ctx.style.text_muted,
        ),
        BrowserPillState::On => active_button_colors(ctx, true),
        BrowserPillState::Mixed => (
            blend_color(
                ctx.style.highlight_orange_soft,
                ctx.style.surface_overlay,
                0.26,
            ),
            blend_color(ctx.style.highlight_orange, ctx.style.text_primary, 0.26),
            ctx.style.text_primary,
        ),
    }
}
