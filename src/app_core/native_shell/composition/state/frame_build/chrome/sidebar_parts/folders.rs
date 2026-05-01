use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;
use native_model::FolderPaneIdModel;

mod header;
mod rows;

pub(super) fn render_folder_section(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) -> usize {
    let sections = sidebar_sections(ctx.layout, ctx.style, ctx.model);
    let mut rendered_count = 0;
    for pane in [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower] {
        render_source_section_divider(ctx, primitives, sections, pane);
        header::render_folder_header(
            ctx,
            primitives,
            text_runs,
            sections.folder_header(pane),
            ctx.model.sources.folder_pane(pane),
        );
        let pane_rows = state.cached_tree_rows(ctx.layout, ctx.style, ctx.model, pane);
        rows::render_tree_rows(ctx, primitives, text_runs, pane, pane_rows);
        if let Some(scrollbar) = folder_scrollbar_layout(
            sections.tree_rows(pane),
            pane_rows,
            ctx.model.sources.folder_pane(pane).tree_rows.len(),
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
        rendered_count += pane_rows.len();
    }
    rendered_count
}

fn render_source_section_divider(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    sections: SidebarSections,
    pane: FolderPaneIdModel,
) {
    let Some(divider_rect) = compute_source_section_divider_rect(
        sections.source_rows(pane),
        sections.folder_header(pane),
        ctx.sizing,
    ) else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: divider_rect,
            color: ctx.style.chrome.source_section_divider,
        }),
    );
}
