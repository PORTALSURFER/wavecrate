use super::*;

pub(super) fn render_browser_frame(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let search_editor_active = state.browser_search_editor_visual.is_some();
    let (buttons, column_chips, toolbar) =
        state.cached_browser_action_hit_test(ctx.layout, ctx.style, ctx.model);

    super::toolbar::render_browser_action_buttons(ctx, primitives, text_runs, buttons);
    super::toolbar::render_browser_column_chips(ctx, primitives, text_runs, column_chips);
    super::toolbar_filters::render_browser_filter_chips(ctx, primitives, &toolbar);
    super::toolbar_filters::render_browser_toolbar_chrome(ctx, primitives, &toolbar);

    let cached_text = state.cached_browser_segment_text(ctx.layout, ctx.style, ctx.model);
    render_browser_tabs(primitives, text_runs, ctx, true, cached_text.as_ref());
    super::toolbar_text::render_browser_toolbar_text(
        ctx,
        text_runs,
        &toolbar,
        cached_text.as_ref(),
        search_editor_active,
    );
}
