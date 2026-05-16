//! State-driven overlay builders for the native shell.

use super::*;
use crate::app_core::native_shell::runtime_contract::FolderRowKind;

#[path = "browser.rs"]
mod browser;
mod focus;
mod waveform;

use self::{
    browser::{
        render_browser_context_menu, render_browser_tab_overlay, render_source_context_menu,
    },
    focus::{
        render_browser_focus_overlay, render_folder_focus_overlay, render_source_focus_overlay,
        render_waveform_focus_overlay,
    },
    waveform::push_waveform_toolbar_hover_tooltip,
};

pub(super) fn push_browser_row_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    focus::push_browser_row_border(primitives, rect, color, stroke, sides);
}

pub(super) fn render_hover_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    push_waveform_toolbar_hover_tooltip(primitives, text_runs, layout, style, model, shell_state);
    let search_field_rect = shell_state.browser_search_field_rect(layout, model);
    let search_text_rect = shell_state.browser_search_text_rect(layout, model);
    if let (Some(search_field_rect), Some(search_text_rect), Some(visual)) = (
        search_field_rect,
        search_text_rect,
        shell_state.browser_search_editor_visual.as_ref(),
    ) {
        render_active_browser_search_editor(
            primitives,
            text_runs,
            style,
            sizing,
            search_field_rect,
            search_text_rect,
            visual,
        );
    }
    let sidebar_input_rect = shell_state.browser_pill_editor_input_rect(layout, model);
    let sidebar_text_rect = shell_state.browser_pill_editor_text_rect(layout, model);
    if let (Some(input_rect), Some(text_rect), Some(visual)) = (
        sidebar_input_rect,
        sidebar_text_rect,
        shell_state.browser_pill_editor_visual.as_ref(),
    ) {
        render_active_text_field(
            primitives,
            text_runs,
            style,
            sizing,
            input_rect,
            text_rect,
            visual,
            browser_search_field_active_fill(style),
            browser_search_field_active_border(style),
            translucent_overlay_color(style.highlight_orange_soft, style.text_primary, 0.22),
            blend_color(style.text_primary, style.highlight_orange, 0.24),
        );
    }
    let folder_input_rect = shell_state.folder_create_input_rect(layout, model);
    let folder_text_rect = shell_state.folder_create_text_rect(layout, model);
    let folder_draft_row = model
        .sources
        .active_folder_pane_model()
        .tree_rows
        .iter()
        .find(|row| row.kind == FolderRowKind::RenameDraft)
        .or_else(|| {
            model
                .sources
                .active_folder_pane_model()
                .tree_rows
                .iter()
                .find(|row| row.kind == FolderRowKind::CreateDraft)
        });
    if let (Some(input_rect), Some(text_rect), Some(draft_row), Some(visual)) = (
        folder_input_rect,
        folder_text_rect,
        folder_draft_row,
        shell_state.folder_create_editor_visual.as_ref(),
    ) {
        render_active_folder_create_editor(
            primitives,
            text_runs,
            style,
            sizing,
            input_rect,
            text_rect,
            visual,
            draft_row
                .input_error
                .as_ref()
                .is_some_and(|error| !error.trim().is_empty()),
        );
    }
    if let Some(hovered_visible_row) = shell_state.hovered_browser_visible_row {
        let browser_rows = shell_state.cached_browser_rows(layout, style, model);
        if let Some(row) = browser_rows
            .iter()
            .find(|row| row.visible_row == hovered_visible_row)
        {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: row.rect,
                    color: browser_row_hover_fill(style),
                }),
            );
        }
    }
    if let (Some(hovered_folder_pane), Some(hovered_folder_row_index)) = (
        shell_state.hovered_folder_pane(),
        shell_state.hovered_folder_row_index,
    ) {
        let tree_rows = shell_state.cached_tree_rows(layout, style, model, hovered_folder_pane);
        if let (Some(row_rect), Some(row)) = (
            tree_rows
                .iter()
                .find(|rendered_row| rendered_row.row_index == hovered_folder_row_index)
                .map(|rendered_row| rendered_row.rect),
            model
                .sources
                .folder_pane(hovered_folder_pane)
                .tree_rows
                .get(hovered_folder_row_index),
        ) && !matches!(
            row.kind,
            FolderRowKind::CreateDraft | FolderRowKind::RenameDraft
        ) {
            let visual_rect = folder_row_visual_rect(row_rect, sizing);
            let color = if model.drag_overlay.active {
                folder_drag_hover_fill(style, model.drag_overlay.valid_target)
            } else {
                subtle_item_hover_fill(style)
            };
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: visual_rect,
                    color,
                }),
            );
        }
    }
}

pub(super) fn render_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    if shell_state.has_focus_emphasis {
        render_waveform_focus_overlay(layout, style, model, primitives);
        render_source_focus_overlay(shell_state, layout, style, model, primitives);
        render_folder_focus_overlay(shell_state, layout, style, model, primitives, text_runs);
        render_browser_focus_overlay(shell_state, layout, style, model, primitives, text_runs);
    }
}

pub(super) fn render_modal_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    render_browser_tab_overlay(primitives, text_runs, layout, style, model);
    render_source_context_menu(
        primitives,
        text_runs,
        layout,
        style,
        model,
        shell_state.source_context_menu,
    );
    render_browser_context_menu(
        primitives,
        text_runs,
        layout,
        style,
        model,
        shell_state.browser_context_menu,
    );
    render_sidebar_filter_dropdown(shell_state, layout, style, model, primitives, text_runs);
    render_progress_overlay(primitives, text_runs, layout, style, model);
    render_confirm_prompt(primitives, text_runs, layout, style, model);
    render_drag_overlay(primitives, text_runs, layout, style, model);
    render_options_panel(
        primitives,
        text_runs,
        layout,
        style,
        model,
        shell_state.options_panel_origin,
    );
}

/// Render the open left-sidebar filter dropdown, if any.
fn render_sidebar_filter_dropdown(
    shell_state: &NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let Some((panel_rect, buttons)) =
        sidebar_filter_dropdown_spec(layout, style, model, shell_state.sidebar_filter_dropdown)
    else {
        return;
    };
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: panel_rect,
            color: blend_color(style.surface_overlay, style.bg_primary, 0.18),
        }),
    );
    push_border(
        primitives,
        panel_rect,
        style.border_emphasis,
        style.sizing.border_width,
    );
    for button in buttons {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: button.rect,
                color: if button.active {
                    blend_color(style.accent_mint, style.surface_overlay, 0.56)
                } else {
                    style.bg_tertiary
                },
            }),
        );
        push_border(
            primitives,
            button.rect,
            if button.active {
                style.accent_mint
            } else {
                style.border
            },
            style.sizing.border_width,
        );
        emit_text(
            text_runs,
            TextRun {
                text: button.label.to_string(),
                position: dropdown_button_text_rect(
                    button.rect,
                    style.sizing.text_inset_x,
                    style.sizing.text_inset_y,
                )
                .min,
                font_size: style.sizing.font_meta,
                color: button.text_color,
                max_width: Some(button.rect.width().max(24.0)),
                align: TextAlign::Left,
            },
        );
    }
}

/// Return a non-inverted text inset for one dropdown option button.
fn dropdown_button_text_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(
            (rect.min.x + x).min(rect.max.x),
            (rect.min.y + y).min(rect.max.y),
        ),
        Point::new(
            (rect.max.x - x).max(rect.min.x),
            (rect.max.y - y).max(rect.min.y),
        ),
    )
}

#[cfg(test)]
pub(super) fn render_state_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    render_hover_overlay(shell_state, layout, style, model, primitives, text_runs);
    render_focus_overlay(shell_state, layout, style, model, primitives, text_runs);
    render_modal_overlay(shell_state, layout, style, model, primitives, text_runs);
}
