use super::rows::BrowserRowRenderCtx;
use super::*;

pub(super) fn render_browser_row_labels_and_inline_tags(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    row: &CachedBrowserRow,
    row_ctx: &BrowserRowRenderCtx,
) {
    emit_text(
        text_runs,
        TextRun {
            text: row.visible_row_label.clone(),
            position: row.text_layout.index_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(row.text_layout.index_label.width().max(12.0)),
            align: TextAlign::Right,
        },
    );
    let (label_position, label_max_width) =
        render_browser_row_sample_label_leading_markers(ctx, text_runs, row, row_ctx);
    let label_max_width =
        render_browser_row_rating_indicators(ctx, primitives, row, label_position, label_max_width);
    emit_text(
        text_runs,
        TextRun {
            text: row.label.clone(),
            position: label_position,
            font_size: ctx.sizing.font_body,
            color: ctx.style.text_primary,
            max_width: Some(label_max_width),
            align: TextAlign::Left,
        },
    );
    render_browser_row_inline_tags(ctx, primitives, text_runs, row);
}

fn render_browser_row_sample_label_leading_markers(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    row: &CachedBrowserRow,
    row_ctx: &BrowserRowRenderCtx,
) -> (Point, f32) {
    let mut label_position = row.text_layout.sample_label.min;
    let mut label_max_width = row.text_layout.sample_label.width().max(20.0);
    reserve_sample_label_prefix(
        row,
        row_ctx.similarity_button_reserved_width,
        &mut label_position,
        &mut label_max_width,
    );
    reserve_sample_label_prefix(
        row,
        row_ctx.age_marker_reserved_width,
        &mut label_position,
        &mut label_max_width,
    );
    if row.missing {
        let marker_advance =
            browser_missing_marker_advance(ctx.sizing.font_body).min(label_max_width.max(0.0));
        emit_text(
            text_runs,
            TextRun {
                text: String::from(BROWSER_MISSING_CONTENT_MARKER),
                position: label_position,
                font_size: ctx.sizing.font_body,
                color: ctx.style.accent_danger,
                max_width: Some(marker_advance),
                align: TextAlign::Left,
            },
        );
        reserve_sample_label_prefix(
            row,
            marker_advance,
            &mut label_position,
            &mut label_max_width,
        );
    }
    (label_position, label_max_width)
}

fn reserve_sample_label_prefix(
    row: &CachedBrowserRow,
    width: f32,
    label_position: &mut Point,
    label_max_width: &mut f32,
) {
    if width <= 0.0 {
        return;
    }
    label_position.x = (label_position.x + width).min(row.text_layout.sample_label.max.x);
    *label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
}

fn render_browser_row_rating_indicators(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    label_position: Point,
    mut label_max_width: f32,
) -> f32 {
    let inline_tag_reserved_width =
        browser_inline_tag_reserved_width_for_labels(&row.inline_tag_labels, ctx.sizing);
    let rating_reserved_width =
        browser_rating_indicator_reserved_width(row.rating_level, row.locked, ctx.sizing);
    let similarity_strength_reserved_width = browser_similarity_strength_reserved_width(
        row.similarity_display_strength.is_some(),
        ctx.sizing,
    );
    let rating_indicator_layout = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row.text_layout.sample_label,
            label_origin_x: label_position.x,
            label_rendered_width: row.label_rendered_width.min(label_max_width.max(0.0)),
            right_limit_x: row.text_layout.sample_label.max.x
                - inline_tag_reserved_width
                - similarity_strength_reserved_width,
        },
        row.rating_level,
        row.locked,
        ctx.sizing,
    );
    if let Some(indicators) = rating_indicator_layout {
        label_max_width = (label_max_width
            - rating_reserved_width
            - inline_tag_reserved_width
            - similarity_strength_reserved_width)
            .max(4.0);
        render_rating_indicator_rects(ctx, primitives, row, indicators);
    } else {
        label_max_width =
            (label_max_width - inline_tag_reserved_width - similarity_strength_reserved_width)
                .max(4.0);
    }
    label_max_width
}

fn render_rating_indicator_rects(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    indicators: BrowserRatingIndicatorLayout,
) {
    for rect in indicators.rects.into_iter().take(indicators.count) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: browser_rating_indicator_color(ctx.style, row.rating_level),
            }),
        );
        push_border(
            primitives,
            rect,
            blend_color(ctx.style.border, ctx.style.text_primary, 0.28),
            ctx.sizing.border_width,
        );
    }
}

fn render_browser_row_inline_tags(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    row: &CachedBrowserRow,
) {
    if row.bucket_label.is_empty() {
        return;
    }
    for (chip_rect, chip_label) in row
        .inline_tag_rects
        .iter()
        .copied()
        .zip(row.inline_tag_labels.iter())
    {
        render_inline_tag(ctx, primitives, text_runs, chip_rect, chip_label);
    }
}

fn render_inline_tag(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    chip_rect: Rect,
    chip_label: &str,
) {
    let text_origin = browser_inline_tag_text_origin(chip_rect, ctx.sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: chip_rect,
            color: blend_color(ctx.style.surface_overlay, ctx.style.bg_tertiary, 0.54),
        }),
    );
    push_border(
        primitives,
        chip_rect,
        blend_color(ctx.style.border_emphasis, ctx.style.text_muted, 0.18),
        ctx.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text: chip_label.to_string(),
            position: text_origin,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some((chip_rect.max.x - text_origin.x).max(4.0)),
            align: TextAlign::Left,
        },
    );
}
