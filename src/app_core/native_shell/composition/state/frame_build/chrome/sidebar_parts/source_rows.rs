use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;
use native_model::{FolderPaneIdModel, SourceRowModel};

pub(super) fn render_source_rows(
    state: &mut NativeShellState,
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) -> usize {
    let mut rendered = 0;
    for rendered_row in state.cached_source_rows(ctx.layout, ctx.style, ctx.model) {
        let Some(row) = ctx.model.sources.rows.get(rendered_row.row_index) else {
            continue;
        };
        let row_selected = source_row_selected(row, rendered_row.pane);
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: rendered_row.rect,
                color: source_row_fill(ctx, row_selected),
            }),
        );
        push_border(
            primitives,
            rendered_row.rect,
            source_row_border(ctx, row, row_selected),
            ctx.sizing.border_width,
        );
        emit_source_row_label(ctx, text_runs, rendered_row.rect, row, row_selected);
        rendered += 1;
    }
    rendered
}

fn source_row_selected(row: &SourceRowModel, pane: FolderPaneIdModel) -> bool {
    match pane {
        FolderPaneIdModel::Upper => row.assigned_to_upper_pane,
        FolderPaneIdModel::Lower => row.assigned_to_lower_pane,
    }
}

fn source_row_fill(ctx: &StaticFrameCtx<'_>, row_selected: bool) -> Rgba8 {
    if row_selected {
        translucent_overlay_color(
            ctx.style.bg_tertiary,
            ctx.style.grid_soft,
            ctx.style.state_selected_blend,
        )
    } else {
        ctx.style.surface_base
    }
}

fn source_row_border(ctx: &StaticFrameCtx<'_>, row: &SourceRowModel, row_selected: bool) -> Rgba8 {
    if row_selected {
        blend_color(
            ctx.style.accent_mint,
            ctx.style.text_primary,
            ctx.motion_wave * ctx.style.state_selected_blend,
        )
    } else if row.missing {
        ctx.style.accent_warning
    } else {
        ctx.style.border
    }
}

fn emit_source_row_label(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    row_rect: Rect,
    row: &SourceRowModel,
    row_selected: bool,
) {
    let label_rect = compute_sidebar_source_row_text_rect(row_rect, ctx.sizing);
    let label_width = label_rect.width().max(24.0);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(&row.label, label_width, ctx.sizing.font_body),
            position: label_rect.min,
            font_size: ctx.sizing.font_body,
            color: if row_selected {
                ctx.style.accent_mint
            } else {
                ctx.style.text_primary
            },
            max_width: Some(label_width),
            align: TextAlign::Left,
        },
    );
}
