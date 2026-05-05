use super::*;

#[path = "sidebar_parts/mod.rs"]
mod parts;

pub(super) fn render_sidebar(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    parts::render_sidebar(state, ctx, primitives, text_runs);
}
