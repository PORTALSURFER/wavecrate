use super::*;

mod folders;
mod footer;
mod header;
mod source_rows;

pub(super) fn render_sidebar(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    header::render_sidebar_header(ctx, primitives, text_runs);
    let rendered_sources = source_rows::render_source_rows(state, ctx, primitives, text_runs);
    let rendered_folders = folders::render_folder_section(state, ctx, primitives, text_runs);
    footer::render_sidebar_footer(
        ctx,
        primitives,
        text_runs,
        rendered_sources,
        rendered_folders,
    );
}
