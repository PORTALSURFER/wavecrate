use super::*;

pub(super) fn render_browser_toolbar_text(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    toolbar: &BrowserToolbarLayout,
    cached_text: &BrowserSegmentTextCacheValue,
    search_editor_active: bool,
) {
    render_search_label(ctx, text_runs, toolbar, cached_text, search_editor_active);
    render_activity_label(ctx, text_runs, toolbar, cached_text);
    render_sort_label(ctx, text_runs, toolbar, cached_text);
}

fn render_search_label(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    toolbar: &BrowserToolbarLayout,
    cached_text: &BrowserSegmentTextCacheValue,
    search_editor_active: bool,
) {
    if toolbar.search_field.width() <= 1.0 || search_editor_active {
        return;
    }
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.search_label.clone(),
            position: cached_text.toolbar_text_layout.search_label.min,
            font_size: ctx.sizing.font_meta,
            color: search_label_color(ctx),
            max_width: Some(
                cached_text
                    .toolbar_text_layout
                    .search_label
                    .width()
                    .max(24.0),
            ),
            align: TextAlign::Left,
        },
    );
}

fn search_label_color(ctx: &StaticFrameCtx<'_>) -> Rgba8 {
    if ctx.model.browser.search_query.is_empty() {
        ctx.style.text_muted
    } else {
        ctx.style.text_primary
    }
}

fn render_activity_label(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    toolbar: &BrowserToolbarLayout,
    cached_text: &BrowserSegmentTextCacheValue,
) {
    if toolbar.activity_chip.width() <= 1.0 {
        return;
    }
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.activity_label.clone(),
            position: cached_text.toolbar_text_layout.activity_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some(
                cached_text
                    .toolbar_text_layout
                    .activity_label
                    .width()
                    .max(20.0),
            ),
            align: TextAlign::Center,
        },
    );
}

fn render_sort_label(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    toolbar: &BrowserToolbarLayout,
    cached_text: &BrowserSegmentTextCacheValue,
) {
    if toolbar.sort_chip.width() <= 1.0 {
        return;
    }
    emit_text(
        text_runs,
        TextRun {
            text: cached_text.sort_label.clone(),
            position: cached_text.toolbar_text_layout.sort_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_muted,
            max_width: Some(cached_text.toolbar_text_layout.sort_label.width().max(20.0)),
            align: TextAlign::Center,
        },
    );
}
