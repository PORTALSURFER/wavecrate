use super::*;

pub(super) fn render_browser_rows_window(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    browser_rows: &[CachedBrowserRow],
) {
    let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
    for row in browser_rows {
        let row_ctx = BrowserRowRenderCtx::new(ctx, row);
        render_browser_row_background_and_decorations(
            ctx,
            primitives,
            row,
            &row_ctx,
            last_row_max_y,
        );
        super::row_labels::render_browser_row_labels_and_inline_tags(
            ctx, primitives, text_runs, row, &row_ctx,
        );
        super::row_similarity::render_browser_row_similarity_controls(
            ctx, primitives, row, &row_ctx,
        );
    }
    render_browser_rows_overlay_chrome(ctx, primitives, text_runs, browser_rows);
}

#[derive(Clone, Copy, Debug)]
/// Stores state for browser row render ctx.
pub(super) struct BrowserRowRenderCtx {
    pub(super) row_border_rect: Rect,
    pub(super) row_border_stroke: f32,
    pub(super) similarity_active: bool,
    pub(super) similarity_button: Option<Rect>,
    pub(super) similarity_button_reserved_width: f32,
    pub(super) similarity_strength_reserved_width: f32,
    pub(super) age_marker_reserved_width: f32,
}

impl BrowserRowRenderCtx {
    /// Handles new.
    fn new(ctx: &StaticFrameCtx<'_>, row: &CachedBrowserRow) -> Self {
        let row_border_stroke = browser_row_border_stroke(ctx.layout);
        let similarity_active =
            ctx.model.browser.similarity_filtered || ctx.model.browser.duplicate_cleanup_active;
        let similarity_button = (!ctx.model.browser.duplicate_cleanup_active)
            .then_some(row)
            .filter(|row| row.focused)
            .and_then(|row| browser_similarity_button_rect(row.rect, ctx.sizing));
        let similarity_button_reserved_width =
            browser_similarity_button_reserved_width(similarity_button.is_some(), ctx.sizing);
        Self {
            row_border_rect: browser_row_border_rect(row.rect, row_border_stroke),
            row_border_stroke,
            similarity_active,
            similarity_button,
            similarity_button_reserved_width,
            similarity_strength_reserved_width: browser_similarity_strength_reserved_width(
                row.similarity_display_strength.is_some(),
                ctx.sizing,
            ),
            age_marker_reserved_width: browser_playback_age_marker_reserved_width(
                row.rect,
                ctx.sizing,
                similarity_button_reserved_width,
            ),
        }
    }
}

/// Handles render browser row background and decorations.
fn render_browser_row_background_and_decorations(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    row_ctx: &BrowserRowRenderCtx,
    last_row_max_y: Option<f32>,
) {
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: row.rect,
            color: browser_row_background_fill(ctx.style, row, row_ctx.similarity_active),
        }),
    );
    render_browser_processing_marker(primitives, row, ctx.style, ctx.sizing);
    render_browser_row_similarity_anchor_marker(ctx, primitives, row, row_ctx);
    render_browser_row_playback_age_marker(ctx, primitives, row, row_ctx);
    render_browser_row_review_markers(ctx, primitives, row);
    render_browser_row_column_separators(ctx, primitives, row);
    push_browser_row_border(
        primitives,
        row_ctx.row_border_rect,
        ctx.style.border,
        row_ctx.row_border_stroke,
        BorderSides {
            top: true,
            bottom: Some(row.rect.max.y) == last_row_max_y,
            left: false,
            right: false,
        },
    );
    render_browser_row_bucket_chip(ctx, primitives, row);
}

/// Handles browser row background fill.
fn browser_row_background_fill(
    style: &StyleTokens,
    row: &CachedBrowserRow,
    similarity_active: bool,
) -> Rgba8 {
    let base_fill = if row.marked && similarity_active {
        browser_marked_similarity_row_fill(style, row.visible_row, row.visible_row == 0)
    } else if row.marked {
        browser_marked_row_fill(style, row.visible_row)
    } else if similarity_active {
        browser_similarity_row_fill(style, row.visible_row, row.visible_row == 0)
    } else {
        browser_row_stripe_fill(style, row.visible_row)
    };
    browser_processing_row_fill(style, base_fill, row.processing_state)
}

/// Handles render browser row similarity anchor marker.
fn render_browser_row_similarity_anchor_marker(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    row_ctx: &BrowserRowRenderCtx,
) {
    if row_ctx.similarity_active && row.visible_row == 0 {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: row.text_layout.columns.index,
                color: similarity_anchor_browser_index_fill(ctx.style),
            }),
        );
    }
}

/// Handles render browser row playback age marker.
fn render_browser_row_playback_age_marker(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    row_ctx: &BrowserRowRenderCtx,
) {
    if let Some(marker_rect) = browser_playback_age_marker_rect(
        row.rect,
        ctx.sizing,
        row_ctx.similarity_button_reserved_width,
    ) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: marker_rect,
                color: browser_playback_age_marker_color(ctx.style, row.playback_age_bucket),
            }),
        );
    }
}

/// Handles render browser row review markers.
fn render_browser_row_review_markers(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
) {
    let marked_marker_width = if row.marked { 4.0 } else { 0.0 };
    if row.marked
        && let Some(marker_rect) = browser_locked_marker_rect(row.rect, ctx.sizing, 0.0)
    {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: marker_rect,
                color: ctx.style.highlight_cyan,
            }),
        );
    }
    if row.locked
        && let Some(marker_rect) =
            browser_locked_marker_rect(row.rect, ctx.sizing, marked_marker_width)
    {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: marker_rect,
                color: ctx.style.accent_mint,
            }),
        );
    }
}

/// Handles render browser row column separators.
fn render_browser_row_column_separators(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
) {
    let row_columns = row.text_layout.columns;
    for separator_x in [row_columns.index.max.x, row_columns.sample.max.x] {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(separator_x, row.rect.min.y),
                    Point::new(
                        (separator_x + ctx.sizing.border_width).min(row.rect.max.x),
                        row.rect.max.y,
                    ),
                ),
                color: blend_color(ctx.style.border, ctx.style.grid_soft, 0.36),
            }),
        );
    }
}

/// Handles render browser row bucket chip.
fn render_browser_row_bucket_chip(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
) {
    let chip_rect = row.text_layout.bucket_chip;
    let chip_color = match row.column {
        0 => blend_color(ctx.style.accent_warning, ctx.style.bg_secondary, 0.54),
        2 => blend_color(ctx.style.accent_mint, ctx.style.bg_secondary, 0.54),
        _ => blend_color(ctx.style.text_muted, ctx.style.bg_secondary, 0.54),
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: chip_rect,
            color: chip_color,
        }),
    );
    push_border(
        primitives,
        chip_rect,
        ctx.style.border,
        ctx.sizing.border_width,
    );
}

/// Handles render browser rows overlay chrome.
fn render_browser_rows_overlay_chrome(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    browser_rows: &[CachedBrowserRow],
) {
    let list_rect = browser_rows_list_rect(ctx.layout.browser_rows, ctx.sizing, ctx.model);
    if let Some(scrollbar) = browser_scrollbar_layout(
        list_rect,
        browser_rows,
        ctx.model.browser.visible_count,
        ctx.sizing,
    ) {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrollbar.track,
                color: blend_color(ctx.style.border, ctx.style.bg_secondary, 0.22),
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: scrollbar.thumb,
                color: blend_color(ctx.style.text_muted, ctx.style.text_primary, 0.32),
            }),
        );
    }
    super::row_overlay::render_browser_pill_editor_overlay(ctx, primitives, text_runs);
}

fn render_browser_processing_marker(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    sizing: SizingTokens,
) {
    let Some(color) = browser_processing_marker_color(style, row.processing_state) else {
        return;
    };
    let marker_width = (sizing.border_width * 3.0).clamp(2.0, 5.0);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                row.rect.min,
                Point::new(
                    (row.rect.min.x + marker_width).min(row.rect.max.x),
                    row.rect.max.y,
                ),
            ),
            color,
        }),
    );
}

#[cfg(test)]
/// Contains focused regression coverage for this module.
mod tests {
    use super::*;
    use crate::app_core::native_shell::runtime_contract::{
        AppModel, BrowserRowModel, BrowserRowProcessingState,
    };
    use crate::gui::types::Vector2;

    /// Handles has fill rect.
    fn has_fill_rect(frame: &NativeViewFrame, rect: Rect, color: Rgba8) -> bool {
        frame.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(FillRect { rect: fill_rect, color: fill_color })
                    if *fill_rect == rect && *fill_color == color
            )
        })
    }

    #[test]
    /// Handles processing browser rows keep processing marker separate from similarity anchor.
    fn processing_browser_rows_keep_processing_marker_separate_from_similarity_anchor() {
        let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
        let style = style_for_layout(&layout);
        let mut state = NativeShellState::new();
        let mut model = AppModel::default();
        model.browser.similarity_filtered = true;
        model.browser.rows.push(
            BrowserRowModel::new(0, "processing anchor", 1, true, true)
                .with_processing_state(BrowserRowProcessingState::Active),
        );
        model.browser.visible_count = model.browser.rows.len();

        let rendered = rendered_browser_rows(&layout, &model, &style);
        let row = rendered.first().expect("browser row should render");
        let frame = state.build_frame(&layout, &model);
        let base_similarity_fill = browser_similarity_row_fill(&style, 0, true);
        let processing_fill = browser_processing_row_fill(
            &style,
            base_similarity_fill,
            BrowserRowProcessingState::Active,
        );
        let marker_width = (style.sizing.border_width * 3.0).clamp(2.0, 5.0);
        let processing_marker_rect = Rect::from_min_max(
            row.rect.min,
            Point::new(
                (row.rect.min.x + marker_width).min(row.rect.max.x),
                row.rect.max.y,
            ),
        );

        assert!(has_fill_rect(&frame, row.rect, processing_fill));
        assert!(!has_fill_rect(&frame, row.rect, base_similarity_fill));
        assert!(has_fill_rect(
            &frame,
            processing_marker_rect,
            style.highlight_orange
        ));
        assert!(has_fill_rect(
            &frame,
            row.text_layout.columns.index,
            similarity_anchor_browser_index_fill(&style)
        ));
    }
}
