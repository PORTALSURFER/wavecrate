use super::*;

pub(super) fn render_browser_table_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let header_text_layout =
        compute_browser_header_text_layout(ctx.layout.browser_table_header, ctx.sizing);
    let header = header_text_layout.columns;
    for separator_x in [header.index.max.x, header.sample.max.x] {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(separator_x, ctx.layout.browser_table_header.min.y),
                    Point::new(
                        (separator_x + ctx.sizing.border_width)
                            .min(ctx.layout.browser_table_header.max.x),
                        ctx.layout.browser_table_header.max.y,
                    ),
                ),
                color: ctx.style.border,
            }),
        );
    }
    emit_text(
        text_runs,
        TextRun {
            text: String::from("#"),
            position: header_text_layout.index_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(header_text_layout.index_label.width().max(12.0)),
            align: TextAlign::Right,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: String::from("Sample"),
            position: header_text_layout.sample_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some(header_text_layout.sample_label.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
}

pub(super) fn render_browser_footer(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
) {
    let cached_text = state.cached_browser_segment_text(ctx.layout, ctx.style, ctx.model);
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.footer_label.clone(),
            position: cached_text.footer_text_rect.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(cached_text.footer_text_rect.width().max(36.0)),
            align: TextAlign::Left,
        },
    );
}

pub(super) fn render_browser_tabs(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    animated: bool,
    cached_text: &BrowserSegmentTextCacheValue,
) {
    let tabs = resolve_browser_tabs_surface_layout(
        ctx.layout.browser_tabs,
        ctx.sizing,
        &browser_tabs_surface_content(ctx.model),
    );
    let wave = if animated { ctx.motion_wave * 0.1 } else { 0.0 };
    let map_active = ctx.model.map.active;
    let (samples_fill, map_fill, samples_border, map_border, samples_text_color, map_text_color) =
        if !map_active {
            (
                blend_color(
                    ctx.style.surface_overlay,
                    ctx.style.bg_tertiary,
                    ctx.style.state_selected_blend + wave,
                ),
                ctx.style.surface_base,
                blend_color(ctx.style.accent_mint, ctx.style.text_primary, 0.42),
                ctx.style.border,
                blend_color(
                    ctx.style.accent_mint,
                    ctx.style.text_primary,
                    ctx.style.state_selected_blend + wave,
                ),
                ctx.style.text_muted,
            )
        } else {
            (
                ctx.style.surface_base,
                blend_color(
                    ctx.style.surface_overlay,
                    ctx.style.bg_tertiary,
                    ctx.style.state_selected_blend + wave,
                ),
                ctx.style.border,
                blend_color(ctx.style.accent_mint, ctx.style.text_primary, 0.42),
                ctx.style.text_muted,
                blend_color(
                    ctx.style.accent_mint,
                    ctx.style.text_primary,
                    ctx.style.state_selected_blend + wave,
                ),
            )
        };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.items,
            color: samples_fill,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tabs.map,
            color: map_fill,
        }),
    );
    push_border(
        primitives,
        tabs.items,
        samples_border,
        ctx.sizing.border_width,
    );
    push_border(primitives, tabs.map, map_border, ctx.sizing.border_width);
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.items_tab_label.clone(),
            position: cached_text.tabs_text_layout.items_label.min,
            font_size: ctx.sizing.font_header,
            color: samples_text_color,
            max_width: Some(cached_text.tabs_text_layout.items_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.map_tab_label.clone(),
            position: cached_text.tabs_text_layout.map_label.min,
            font_size: ctx.sizing.font_header,
            color: map_text_color,
            max_width: Some(cached_text.tabs_text_layout.map_label.width().max(40.0)),
            align: TextAlign::Left,
        },
    );
}
