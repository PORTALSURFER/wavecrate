use super::rows::BrowserRowRenderCtx;
use super::*;

pub(super) fn render_browser_row_similarity_controls(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    row_ctx: &BrowserRowRenderCtx,
) {
    render_browser_row_similarity_strength(ctx, primitives, row, row_ctx);
    if let Some(button_rect) = row_ctx.similarity_button {
        let button_active = row_ctx.similarity_active && row.visible_row == 0;
        render_browser_similarity_button(
            primitives,
            button_rect,
            ctx.style,
            ctx.sizing,
            button_active,
            if button_active {
                ctx.style.text_primary
            } else {
                ctx.style.text_muted
            },
        );
    }
}

fn render_browser_row_similarity_strength(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    row_ctx: &BrowserRowRenderCtx,
) {
    if row_ctx.similarity_strength_reserved_width <= 0.0 {
        return;
    }
    let Some(strength) = row.similarity_display_strength else {
        return;
    };
    let Some(track_rect) =
        browser_similarity_strength_track_rect(row.text_layout.sample_label, ctx.sizing)
    else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: track_rect,
            color: translucent_overlay_color(ctx.style.surface_overlay, ctx.style.text_muted, 0.12),
        }),
    );
    if let Some(fill_rect) = browser_similarity_strength_fill_rect(track_rect, strength) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: fill_rect,
                color: blend_color(
                    ctx.style.highlight_cyan_soft,
                    ctx.style.highlight_cyan,
                    0.38,
                ),
            }),
        );
    }
}
