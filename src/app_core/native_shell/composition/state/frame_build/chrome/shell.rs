use super::*;

pub(super) fn render_static_shell_surfaces(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
) {
    for (rect, color) in [
        (ctx.layout.top_bar, ctx.style.surface_raised),
        (ctx.layout.sidebar, ctx.style.surface_raised),
        (ctx.layout.content, ctx.style.surface_base),
        (ctx.layout.waveform_card, ctx.style.surface_raised),
        (ctx.layout.status_bar, ctx.style.surface_raised),
        (ctx.layout.browser_panel, ctx.style.surface_raised),
        (ctx.layout.browser_tabs, ctx.style.surface_overlay),
        (ctx.layout.browser_toolbar, ctx.style.surface_overlay),
        (ctx.layout.browser_table_header, ctx.style.surface_overlay),
        (ctx.layout.browser_footer, ctx.style.surface_overlay),
    ] {
        emit_primitive(primitives, Primitive::Rect(FillRect { rect, color }));
    }
}

pub(super) fn render_shell_borders(ctx: &StaticFrameCtx<'_>, primitives: &mut impl PrimitiveSink) {
    push_border_sides(
        primitives,
        ctx.layout.top_bar,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides::ALL,
    );
    push_border_sides(
        primitives,
        ctx.layout.sidebar,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: false,
            bottom: false,
            left: true,
            right: true,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.waveform_card,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: false,
            bottom: true,
            left: false,
            right: true,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.browser_panel,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: false,
            bottom: false,
            left: false,
            right: true,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.browser_table_header,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: true,
            bottom: true,
            left: false,
            right: false,
        },
    );
    push_border_sides(
        primitives,
        ctx.layout.status_bar,
        ctx.style.border,
        ctx.sizing.border_width,
        BorderSides {
            top: true,
            bottom: false,
            left: true,
            right: true,
        },
    );
}
