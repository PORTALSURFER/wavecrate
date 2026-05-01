use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;
use native_model::FolderPaneModel;

pub(super) fn render_folder_header(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    header_rect: Rect,
    pane: &FolderPaneModel,
) {
    let header_layout = compute_sidebar_folder_header_layout(
        header_rect,
        ctx.sizing,
        pane.recovery.in_progress,
        pane.recovery.entry_count,
        pane.show_all_items,
        pane.can_toggle_show_all_items,
        pane.flattened_view,
        pane.can_toggle_flattened_view,
    );
    render_folder_header_toggle_button(
        ctx,
        primitives,
        header_layout
            .visibility_toggle_button
            .as_ref()
            .map(|button| (button.rect, button.active, button.enabled)),
        WaveformToolbarIcon::Filter,
    );
    render_folder_header_toggle_button(
        ctx,
        primitives,
        header_layout
            .flatten_toggle_button
            .as_ref()
            .map(|button| (button.rect, button.active, button.enabled)),
        WaveformToolbarIcon::Flatten,
    );
    if let Some(badge) = header_layout.badge.as_ref() {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: badge.rect,
                color: if badge.active {
                    ctx.style.chrome.source_recovery_badge_active
                } else {
                    ctx.style.chrome.source_recovery_badge_idle
                },
            }),
        );
        push_border(
            primitives,
            badge.rect,
            blend_color(
                ctx.style.border_emphasis,
                ctx.style.text_primary,
                ctx.style.state_hover_soft,
            ),
            ctx.sizing.border_width,
        );
        let badge_text_rect = compute_sidebar_recovery_badge_text_rect(badge.rect, ctx.sizing);
        emit_text(
            text_runs,
            TextRun {
                text: badge.label.clone(),
                position: badge_text_rect.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_primary,
                max_width: Some(badge_text_rect.width().max(18.0)),
                align: TextAlign::Center,
            },
        );
    }
    if header_layout.title_row.width() <= 8.0 {
        return;
    }
    let title_prefix = if pane.active {
        "Active"
    } else {
        pane.title.as_str()
    };
    emit_text(
        text_runs,
        TextRun {
            text: format!(
                "{} Pane: {} ({})",
                title_prefix,
                pane.item_label,
                pane.tree_rows.len()
            ),
            position: header_layout.title_row.min,
            font_size: ctx.sizing.font_header,
            color: if pane.active {
                ctx.style.accent_mint
            } else {
                ctx.style.text_primary
            },
            max_width: Some(header_layout.title_row.width()),
            align: TextAlign::Left,
        },
    );
    if let Some(metadata_row) = header_layout.metadata_row {
        if metadata_row.width() <= 24.0 {
            return;
        }
        emit_text(
            text_runs,
            TextRun {
                text: format!(
                    "{} | query: {}",
                    if pane.item_detail.is_empty() {
                        "no source"
                    } else {
                        pane.item_detail.as_str()
                    },
                    if pane.tree_search_query.is_empty() {
                        "—"
                    } else {
                        pane.tree_search_query.as_str()
                    }
                ),
                position: metadata_row.min,
                font_size: ctx.sizing.font_meta,
                color: ctx.style.text_muted,
                max_width: Some(metadata_row.width()),
                align: TextAlign::Left,
            },
        );
    }
}

fn render_folder_header_toggle_button(
    ctx: &StaticFrameCtx<'_>,
    primitives: &mut impl PrimitiveSink,
    toggle_button: Option<(Rect, bool, bool)>,
    icon: WaveformToolbarIcon,
) {
    let Some((toggle_rect, active, enabled)) = toggle_button else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: toggle_rect,
            color: folder_toggle_fill(ctx, active, enabled),
        }),
    );
    push_border(
        primitives,
        toggle_rect,
        folder_toggle_border(ctx, active, enabled),
        ctx.sizing.border_width,
    );
    if let Some(icon_rect) = centered_header_toggle_icon_rect(toggle_rect) {
        let _ = emit_toolbar_svg_icon(
            primitives,
            icon,
            icon_rect,
            folder_toggle_text_color(ctx, active, enabled),
        );
    }
}

fn centered_header_toggle_icon_rect(button_rect: Rect) -> Option<Rect> {
    let size = (button_rect.width().min(button_rect.height()) - 6.0)
        .floor()
        .clamp(8.0, 14.0);
    if size <= 0.0 {
        return None;
    }
    let min_x = button_rect.min.x + ((button_rect.width() - size) * 0.5).floor();
    let min_y = button_rect.min.y + ((button_rect.height() - size) * 0.5).floor();
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + size, min_y + size),
    ))
}

fn folder_toggle_fill(ctx: &StaticFrameCtx<'_>, active: bool, enabled: bool) -> Rgba8 {
    if !enabled {
        return blend_color(ctx.style.surface_base, ctx.style.border, 0.18);
    }
    if active {
        blend_color(ctx.style.surface_overlay, ctx.style.accent_mint, 0.22)
    } else {
        ctx.style.surface_overlay
    }
}

fn folder_toggle_border(ctx: &StaticFrameCtx<'_>, active: bool, enabled: bool) -> Rgba8 {
    if !enabled {
        return ctx.style.border;
    }
    if active {
        blend_color(ctx.style.border_emphasis, ctx.style.accent_mint, 0.52)
    } else {
        blend_color(
            ctx.style.border_emphasis,
            ctx.style.text_primary,
            ctx.style.state_hover_soft,
        )
    }
}

fn folder_toggle_text_color(ctx: &StaticFrameCtx<'_>, active: bool, enabled: bool) -> Rgba8 {
    if !enabled {
        return ctx.style.text_muted;
    }
    if active {
        ctx.style.accent_mint
    } else {
        ctx.style.text_primary
    }
}
