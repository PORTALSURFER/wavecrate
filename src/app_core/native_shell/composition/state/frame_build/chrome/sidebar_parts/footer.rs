use super::*;

pub(super) fn render_sidebar_footer(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    rendered_sources: usize,
    rendered_folders: usize,
) {
    let content = sidebar_footer_surface_content(ctx.model, rendered_sources, rendered_folders);
    let surface =
        resolve_sidebar_footer_surface_layout(ctx.layout.sidebar_footer, ctx.sizing, &content);
    render_source_action_buttons(ctx, primitives, text_runs);
    render_sidebar_footer_text(ctx, text_runs, &content, &surface);
}

fn render_source_action_buttons(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    for button in source_action_buttons(ctx.layout, ctx.style, ctx.model) {
        let label_rect = compute_action_button_text_rect(button.rect, ctx.sizing);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.enabled {
                    ctx.style.surface_overlay
                } else {
                    ctx.style.control_disabled_fill
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.enabled {
                blend_color(
                    ctx.style.border_emphasis,
                    ctx.style.text_primary,
                    ctx.style.state_hover_soft,
                )
            } else {
                ctx.style.border
            },
            ctx.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.label.to_string(),
                position: label_rect.min,
                font_size: ctx.sizing.font_meta,
                color: if button.enabled {
                    button.text_color
                } else {
                    ctx.style.text_muted
                },
                max_width: Some(label_rect.width().max(12.0)),
                align: TextAlign::Center,
            },
        );
    }
}

fn render_sidebar_footer_text(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    content: &SidebarFooterSurfaceContent,
    surface: &SidebarFooterSurfaceLayout,
) {
    if !content.primary_summary.is_empty() {
        emit_text(
            text_runs,
            TextRun {
                text: content.primary_summary.clone(),
                position: surface.primary_text_rect.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(surface.primary_text_rect.width().max(56.0)),
                align: TextAlign::Left,
            },
        );
    }
    if !content.secondary_summary.is_empty() {
        emit_text(
            text_runs,
            TextRun {
                text: content.secondary_summary.clone(),
                position: surface.secondary_text_rect.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(surface.secondary_text_rect.width().max(56.0)),
                align: TextAlign::Left,
            },
        );
    }
}
