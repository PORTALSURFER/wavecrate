use super::*;
use crate::app_core::native_shell::runtime_contract::{FocusContextModel, FolderPaneIdModel};

mod browser;
mod shared;

pub(super) use self::browser::render_browser_focus_overlay;
use self::shared::{render_section_focus_surface, union_rect};

pub(super) fn push_browser_row_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    self::shared::push_browser_row_border(primitives, rect, color, stroke, sides);
}

fn render_sidebar_section_focus_overlay(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
) {
    let sections = sidebar_sections(layout, style, model);
    match model.focus_context {
        FocusContextModel::NavigationList => {
            render_section_focus_surface(
                primitives,
                sections.source_rows(model.sources.active_folder_pane),
                style,
            );
        }
        FocusContextModel::NavigationTree => {
            let active_pane = model.sources.active_folder_pane;
            render_section_focus_surface(
                primitives,
                union_rect(
                    sections.folder_header(active_pane),
                    sections.tree_rows(active_pane),
                ),
                style,
            );
        }
        _ => {}
    }
}

pub(super) fn render_waveform_focus_overlay(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
) {
    if matches!(model.focus_context, FocusContextModel::Timeline) {
        render_section_focus_surface(primitives, layout.waveform_card, style);
    }
}

fn render_panel_focus_surface(
    rect: Rect,
    style: &StyleTokens,
    primitives: &mut impl PrimitiveSink,
) {
    render_section_focus_surface(primitives, rect, style);
}

pub(super) fn render_source_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
) {
    if matches!(model.focus_context, FocusContextModel::NavigationList) {
        render_sidebar_section_focus_overlay(layout, style, model, primitives);
    }
    let source_rows = shell_state
        .cached_source_rows(layout, style, model)
        .to_vec();
    for rendered_row in source_rows {
        let Some(row) = model.sources.rows.get(rendered_row.row_index) else {
            continue;
        };
        let row_selected = match rendered_row.pane {
            FolderPaneIdModel::Upper => row.assigned_to_upper_pane,
            FolderPaneIdModel::Lower => row.assigned_to_lower_pane,
        };
        if !row_selected {
            continue;
        }
        push_border(
            primitives,
            rendered_row.rect,
            blend_color(
                style.accent_mint,
                style.text_primary,
                style.state_selected_blend,
            ),
            style.sizing.border_width,
        );
    }
}

pub(super) fn render_folder_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    if matches!(model.focus_context, FocusContextModel::NavigationTree) {
        render_sidebar_section_focus_overlay(layout, style, model, primitives);
    }
    for pane in [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower] {
        let pane_rows = shell_state.cached_tree_rows(layout, style, model, pane);
        let pane_model = model.sources.folder_pane(pane);
        let last_folder_row_max_y = pane_rows.last().map(|row| row.rect.max.y);
        for rendered_row in pane_rows.iter() {
            let Some(row) = pane_model.tree_rows.get(rendered_row.row_index) else {
                continue;
            };
            let row_rect = rendered_row.rect;
            let visual_rect = folder_row_visual_rect(row_rect, sizing);
            if !(row.selected || row.focused) {
                continue;
            }
            if row.focused {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: visual_rect,
                        color: translucent_overlay_color(
                            style.bg_tertiary,
                            style.grid_strong,
                            style.state_focus_pulse_blend,
                        ),
                    }),
                );
            }
            if row.focused || row.selected {
                push_browser_row_border(
                    primitives,
                    visual_rect,
                    if row.focused {
                        blend_color(
                            style.accent_warning,
                            style.text_primary,
                            style.state_focus_pulse_blend,
                        )
                    } else {
                        blend_color(
                            style.accent_mint,
                            style.text_primary,
                            style.state_selected_blend,
                        )
                    },
                    if row.focused {
                        sizing.focus_stroke_width
                    } else {
                        sizing.border_width
                    },
                    BorderSides {
                        top: true,
                        bottom: row.focused || Some(visual_rect.max.y) == last_folder_row_max_y,
                        left: row.focused,
                        right: row.focused,
                    },
                );
            }
            if row.focused {
                let depth_indent =
                    compute_sidebar_folder_row_depth_indent(row_rect, sizing, row.depth);
                let row_text_rect =
                    compute_sidebar_folder_row_layout(row_rect, sizing, depth_indent).label_rect;
                let row_text_width = row_text_rect.width().max(24.0);
                emit_text(
                    text_runs,
                    TextRun {
                        text: truncate_to_width(&row.label, row_text_width, sizing.font_body),
                        position: row_text_rect.min,
                        font_size: sizing.font_body,
                        color: blend_color(
                            style.accent_warning,
                            style.text_primary,
                            style.state_focus_pulse_blend,
                        ),
                        max_width: Some(row_text_width),
                        align: TextAlign::Left,
                    },
                );
            }
        }
    }
}
