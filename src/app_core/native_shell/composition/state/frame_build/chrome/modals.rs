use super::*;

pub(super) fn render_modal_overlays(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) {
    render_progress_overlay(primitives, text_runs, layout, style, model);
    render_confirm_prompt(primitives, text_runs, layout, style, model);
    render_drag_overlay(primitives, text_runs, layout, style, model);
    render_options_panel(primitives, text_runs, layout, style, model);
}
