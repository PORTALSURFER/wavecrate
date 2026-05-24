use super::*;
use crate::app_core::native_shell::runtime_contract::FocusContextModel;

mod labels;
mod markers;

use self::labels::render_browser_row_focus_content;
use self::markers::render_browser_row_markers;

pub(in crate::app_core::native_shell::composition::state::frame_build::overlay) fn render_browser_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    if matches!(model.focus_context, FocusContextModel::ContentList) {
        super::render_panel_focus_surface(layout.browser_panel, style, primitives);
    }

    let browser_rows = shell_state
        .cached_browser_rows(layout, style, model)
        .to_vec();
    let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
    let row_border_stroke = browser_row_border_stroke(layout);
    for row in browser_rows
        .iter()
        .filter(|row| row.selected || row.focused)
    {
        render_browser_row_surface(primitives, row, style, model);
        render_browser_row_markers(primitives, row, style, model, row_border_stroke);
        render_browser_row_border(primitives, row, style, row_border_stroke, last_row_max_y);
        render_browser_row_focus_content(primitives, text_runs, row, style, model);
    }
}

fn render_browser_row_surface(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    model: &AppModel,
) {
    if row.focused {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: row.rect,
                color: translucent_overlay_color(
                    style.bg_tertiary,
                    style.grid_strong,
                    style.state_focus_pulse_blend,
                ),
            }),
        );
    } else if row.selected {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: row.rect,
                color: selected_browser_row_fill(style),
            }),
        );
    }

    if row.selected {
        emit_selected_browser_index(primitives, row, style, model);
    }
}

fn emit_selected_browser_index(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    model: &AppModel,
) {
    let similarity_anchor_active = (model.browser.similarity_filtered
        || model.browser.duplicate_cleanup_active)
        && row.visible_row == 0;
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: row.text_layout.columns.index,
            color: if similarity_anchor_active {
                similarity_anchor_browser_index_fill(style)
            } else {
                selected_browser_index_fill(style)
            },
        }),
    );
}

fn render_browser_row_border(
    primitives: &mut impl PrimitiveSink,
    row: &CachedBrowserRow,
    style: &StyleTokens,
    row_border_stroke: f32,
    last_row_max_y: Option<f32>,
) {
    super::push_browser_row_border(
        primitives,
        browser_row_border_rect(row.rect, row_border_stroke),
        if row.focused {
            focused_browser_row_color(style)
        } else {
            style.border
        },
        row_border_stroke,
        BorderSides {
            top: true,
            bottom: row.focused || Some(row.rect.max.y) == last_row_max_y,
            left: row.focused,
            right: row.focused,
        },
    );
}

pub(super) fn focused_browser_row_color(style: &StyleTokens) -> Rgba8 {
    blend_color(
        style.accent_warning,
        style.text_primary,
        style.state_focus_pulse_blend,
    )
}
