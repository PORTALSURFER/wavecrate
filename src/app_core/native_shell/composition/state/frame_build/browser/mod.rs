use super::StaticFrameCtx;
use super::*;

mod panel;
/// Browser row label, rating, and inline tag rendering.
mod row_labels;
/// Browser row overlay editor rendering.
mod row_overlay;
/// Browser row similarity controls.
mod row_similarity;
mod rows;
mod tabs;
/// Browser toolbar button and column-chip rendering.
mod toolbar;
/// Browser toolbar filter-chip rendering.
mod toolbar_filters;
/// Browser toolbar text rendering.
mod toolbar_text;

pub(super) fn render_browser_frame(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    panel::render_browser_frame(state, ctx, primitives, text_runs);
}

pub(super) fn render_browser_rows_window(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let browser_rows = state.cached_browser_rows(ctx.layout, ctx.style, ctx.model);
    rows::render_browser_rows_window(ctx, primitives, text_runs, browser_rows);
}

pub(super) fn render_browser_footer(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
) {
    tabs::render_browser_footer(state, ctx, text_runs);
}

pub(super) fn render_browser_table_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    tabs::render_browser_table_header(ctx, primitives, text_runs);
}

pub(super) fn render_browser_tabs(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    ctx: &StaticFrameCtx<'_>,
    animated: bool,
    cached_text: &BrowserSegmentTextCacheValue,
) {
    tabs::render_browser_tabs(primitives, text_runs, ctx, animated, cached_text);
}

/// Keep icon-only browser toolbar controls visually centered in fixed-size hit targets.
fn centered_button_icon_rect(button_rect: Rect, sizing: SizingTokens) -> Rect {
    let side = button_rect
        .width()
        .min(button_rect.height())
        .min((button_rect.height() - (sizing.text_inset_y * 0.8)).max(8.0))
        .clamp(8.0, 20.0);
    let min_x = button_rect.min.x + ((button_rect.width() - side) * 0.5);
    let min_y = button_rect.min.y + ((button_rect.height() - side) * 0.5);
    Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + side, min_y + side),
    )
}
