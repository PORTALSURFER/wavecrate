use super::StaticFrameCtx;
use super::*;

mod panel;
mod rows;
mod tabs;

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
