use super::*;
use crate::app::{FolderPaneIdModel, FolderRowModel};

pub(super) fn render_folder_rows(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    pane: FolderPaneIdModel,
    rows: &[CachedFolderRow],
) {
    let pane_model = ctx.model.sources.folder_pane(pane);
    let last_row_max_y = rows.last().map(|row| row.rect.max.y);
    for rendered_row in rows {
        let Some(row) = pane_model.folder_rows.get(rendered_row.row_index) else {
            continue;
        };
        let row_rect = rendered_row.rect;
        let visual_rect = folder_row_visual_rect(row_rect, ctx.sizing);
        if matches!(
            row.kind,
            crate::app::FolderRowKind::CreateDraft | crate::app::FolderRowKind::RenameDraft
        ) {
            render_folder_inline_draft_row(ctx, primitives, text_runs, row_rect, visual_rect, row);
            continue;
        }
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: visual_rect,
                color: folder_row_fill(ctx, row),
            }),
        );
        push_browser_row_border(
            primitives,
            visual_rect,
            folder_row_border(ctx, row),
            folder_row_border_width(ctx, row),
            BorderSides {
                top: true,
                bottom: row.focused || Some(visual_rect.max.y) == last_row_max_y,
                left: row.focused,
                right: row.focused,
            },
        );
        emit_folder_row_disclosure(ctx, primitives, row_rect, row);
        emit_folder_row_label(ctx, text_runs, row_rect, row);
    }
}

fn render_folder_inline_draft_row(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    row_rect: Rect,
    visual_rect: Rect,
    row: &FolderRowModel,
) {
    let field_rect = create_draft_field_rect(row_rect, ctx.sizing, row.depth);
    let text_rect = create_draft_text_rect(field_rect, ctx.sizing);
    let has_error = row
        .input_error
        .as_deref()
        .is_some_and(|error| !error.trim().is_empty());
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: visual_rect,
            color: ctx.style.surface_base,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: field_rect,
            color: browser_search_field_active_fill(ctx.style),
        }),
    );
    push_border(
        primitives,
        field_rect,
        if has_error {
            blend_color(ctx.style.accent_copper, ctx.style.text_primary, 0.45)
        } else {
            browser_search_field_active_border(ctx.style)
        },
        ctx.sizing.border_width,
    );
    let text = row.input_value.as_deref().unwrap_or_default();
    let (display_text, color) = if text.is_empty() {
        (
            row.input_placeholder
                .as_deref()
                .unwrap_or("New folder name")
                .to_string(),
            if has_error {
                blend_color(ctx.style.accent_copper, ctx.style.text_primary, 0.35)
            } else {
                ctx.style.text_muted
            },
        )
    } else {
        (
            text.to_string(),
            if has_error {
                blend_color(ctx.style.accent_copper, ctx.style.text_primary, 0.18)
            } else {
                ctx.style.text_primary
            },
        )
    };
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(
                &display_text,
                text_rect.width().max(24.0),
                ctx.sizing.font_meta,
            ),
            position: text_rect.min,
            font_size: ctx.sizing.font_meta,
            color,
            max_width: Some(text_rect.width().max(24.0)),
            align: TextAlign::Left,
        },
    );
}

fn folder_row_fill(ctx: &StaticFrameCtx<'_>, row: &FolderRowModel) -> Rgba8 {
    if row.focused {
        translucent_overlay_color(
            ctx.style.bg_tertiary,
            ctx.style.grid_strong,
            (ctx.style.state_hover_soft + (ctx.motion_wave * ctx.style.motion_focus_wave_amp))
                .clamp(0.0, 1.0),
        )
    } else if row.selected {
        translucent_overlay_color(
            ctx.style.bg_tertiary,
            ctx.style.grid_soft,
            ctx.style.state_selected_blend,
        )
    } else {
        ctx.style.surface_base
    }
}

fn folder_row_border(ctx: &StaticFrameCtx<'_>, row: &FolderRowModel) -> Rgba8 {
    if row.focused {
        blend_color(
            ctx.style.accent_warning,
            ctx.style.text_primary,
            ctx.motion_wave * ctx.style.state_focus_pulse_blend,
        )
    } else if row.selected {
        blend_color(
            ctx.style.accent_mint,
            ctx.style.text_primary,
            ctx.motion_wave * ctx.style.state_selected_blend,
        )
    } else {
        ctx.style.border
    }
}

fn folder_row_border_width(ctx: &StaticFrameCtx<'_>, row: &FolderRowModel) -> f32 {
    if row.focused {
        ctx.sizing.focus_stroke_width
    } else {
        ctx.sizing.border_width
    }
}

fn folder_row_text_color(ctx: &StaticFrameCtx<'_>, row: &FolderRowModel) -> Rgba8 {
    if row.focused {
        ctx.style.accent_warning
    } else if row.selected {
        ctx.style.accent_mint
    } else {
        ctx.style.text_primary
    }
}

fn emit_folder_row_disclosure(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    row_rect: Rect,
    row: &FolderRowModel,
) {
    if row.is_root || !row.has_children {
        return;
    }
    let layout = folder_row_layout(ctx, row_rect, row);
    let Some(icon_rect) = centered_tree_icon_rect(layout.disclosure_rect) else {
        return;
    };
    if row.expanded {
        emit_down_triangle(primitives, icon_rect, folder_row_text_color(ctx, row));
    } else {
        emit_right_triangle(primitives, icon_rect, folder_row_text_color(ctx, row));
    }
}

fn emit_folder_row_label(
    ctx: &StaticFrameCtx<'_>,
    text_runs: &mut impl TextRunSink,
    row_rect: Rect,
    row: &FolderRowModel,
) {
    let label_rect = folder_row_layout(ctx, row_rect, row).label_rect;
    let label_width = label_rect.width().max(24.0);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(&row.label, label_width, ctx.sizing.font_body),
            position: label_rect.min,
            font_size: ctx.sizing.font_body,
            color: folder_row_text_color(ctx, row),
            max_width: Some(label_width),
            align: TextAlign::Left,
        },
    );
}

fn folder_row_layout(
    ctx: &StaticFrameCtx<'_>,
    row_rect: Rect,
    row: &FolderRowModel,
) -> SidebarFolderRowLayout {
    let depth_indent = compute_sidebar_folder_row_depth_indent(row_rect, ctx.sizing, row.depth);
    compute_sidebar_folder_row_layout(row_rect, ctx.sizing, depth_indent)
}

fn centered_tree_icon_rect(gutter_rect: Rect) -> Option<Rect> {
    if gutter_rect.width() <= 1.0 || gutter_rect.height() <= 1.0 {
        return None;
    }
    let mut size = gutter_rect
        .width()
        .min(gutter_rect.height())
        .floor()
        .clamp(5.0, 9.0);
    if (size as i32) % 2 == 0 {
        size -= 1.0;
    }
    if size < 5.0 {
        return None;
    }
    let min_x = gutter_rect.min.x + ((gutter_rect.width() - size) * 0.5).floor();
    let min_y = gutter_rect.min.y + ((gutter_rect.height() - size) * 0.5).floor();
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + size, min_y + size),
    ))
}

fn emit_right_triangle(primitives: &mut impl PrimitiveSink, rect: Rect, color: Rgba8) {
    let size = rect.width().min(rect.height()).floor() as i32;
    let half = size / 2;
    for step in 0..=half {
        let x = rect.min.x + step as f32;
        let y = rect.min.y + step as f32;
        let height = (size - (step * 2)) as f32;
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(x, y), Point::new(x + 1.0, y + height)),
                color,
            }),
        );
    }
}

fn emit_down_triangle(primitives: &mut impl PrimitiveSink, rect: Rect, color: Rgba8) {
    let size = rect.width().min(rect.height()).floor() as i32;
    let half = size / 2;
    for step in 0..=half {
        let x = rect.min.x + step as f32;
        let y = rect.min.y + step as f32;
        let width = (size - (step * 2)) as f32;
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(x, y), Point::new(x + width, y + 1.0)),
                color,
            }),
        );
    }
}

fn create_draft_field_rect(row_rect: Rect, sizing: SizingTokens, depth: usize) -> Rect {
    let depth_indent = compute_sidebar_folder_row_depth_indent(row_rect, sizing, depth);
    let label_rect = compute_sidebar_folder_row_layout(row_rect, sizing, depth_indent).label_rect;
    let horizontal_inset = sizing.text_inset_x.max(4.0) * 0.5;
    let vertical_inset = sizing.text_inset_y.max(2.0) * 0.5;
    Rect::from_min_max(
        Point::new(
            (label_rect.min.x - horizontal_inset).max(row_rect.min.x),
            row_rect.min.y + vertical_inset,
        ),
        Point::new(
            row_rect.max.x - horizontal_inset,
            row_rect.max.y - vertical_inset,
        ),
    )
}

fn create_draft_text_rect(field_rect: Rect, sizing: SizingTokens) -> Rect {
    compute_action_button_text_rect(field_rect, sizing)
}
