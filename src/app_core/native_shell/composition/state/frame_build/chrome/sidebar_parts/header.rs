use super::*;

pub(super) fn render_sidebar_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let content = sidebar_header_surface_content(ctx.model);
    let surface =
        resolve_sidebar_header_surface_layout(ctx.layout.sidebar_header, ctx.sizing, &content);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &content.title,
                surface.title_text_rect.width().max(24.0),
                ctx.sizing.font_header,
            ),
            position: surface.title_text_rect.min,
            font_size: ctx.sizing.font_header,
            color: ctx.style.text_primary,
            max_width: Some(surface.title_text_rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: content.query,
            position: surface.query_text_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(surface.query_text_rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
    render_source_add_button(ctx, primitives, text_runs, surface.add_button_rect);
}

fn render_source_add_button(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    button_rect: Option<Rect>,
) {
    let Some(button_rect) = button_rect else {
        return;
    };
    let label_rect = compute_action_button_text_rect(button_rect, ctx.sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: button_rect,
            color: ctx.style.surface_overlay,
        }),
    );
    push_border(
        primitives,
        button_rect,
        blend_color(
            ctx.style.border_emphasis,
            ctx.style.text_primary,
            ctx.style.state_hover_soft,
        ),
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("+"),
            position: label_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.accent_mint,
            max_width: Some(label_rect.width().max(8.0)),
            align: TextAlign::Center,
        },
    );
}
