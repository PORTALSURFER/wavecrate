use super::StaticFrameCtx;
use super::*;

mod modals;
mod shell;
#[path = "sidebar.rs"]
mod sidebar;
mod top_bar;

pub(super) fn render_modal_overlays(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    modals::render_modal_overlays(primitives, text_runs, layout, style, model);
}

pub(super) fn render_shell_borders(ctx: &StaticFrameCtx<'_>, primitives: &mut impl PrimitiveSink) {
    shell::render_shell_borders(ctx, primitives);
}

pub(super) fn render_static_shell_surfaces(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
) {
    shell::render_static_shell_surfaces(ctx, primitives);
}

pub(super) fn render_sidebar(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    sidebar::render_sidebar(state, ctx, primitives, text_runs);
}

pub(super) fn render_top_bar_controls(
    state: &NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    top_bar::render_top_bar_controls(state, ctx, primitives, text_runs);
}
