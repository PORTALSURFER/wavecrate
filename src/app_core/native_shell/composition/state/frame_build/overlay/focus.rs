use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;

mod shared;

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
        native_model::FocusContextModel::NavigationList => {
            render_section_focus_surface(
                primitives,
                sections.source_rows(model.sources.active_folder_pane),
                style,
            );
        }
        native_model::FocusContextModel::NavigationTree => {
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
    if matches!(
        model.focus_context,
        native_model::FocusContextModel::Timeline
    ) {
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
    if matches!(
        model.focus_context,
        native_model::FocusContextModel::NavigationList
    ) {
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
            native_model::FolderPaneIdModel::Upper => row.assigned_to_upper_pane,
            native_model::FolderPaneIdModel::Lower => row.assigned_to_lower_pane,
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
    if matches!(
        model.focus_context,
        native_model::FocusContextModel::NavigationTree
    ) {
        render_sidebar_section_focus_overlay(layout, style, model, primitives);
    }
    for pane in [
        native_model::FolderPaneIdModel::Upper,
        native_model::FolderPaneIdModel::Lower,
    ] {
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

pub(super) fn render_browser_focus_overlay(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
) {
    let sizing = style.sizing;
    if matches!(
        model.focus_context,
        native_model::FocusContextModel::ContentList
    ) {
        render_panel_focus_surface(layout.browser_panel, style, primitives);
    }
    let browser_rows = shell_state.cached_browser_rows(layout, style, model);
    let last_row_max_y = browser_rows.last().map(|row| row.rect.max.y);
    for row in browser_rows.iter() {
        if !(row.selected || row.focused) {
            continue;
        }
        let similarity_anchor_active = (model.browser.similarity_filtered
            || model.browser.duplicate_cleanup_active)
            && row.visible_row == 0;
        let row_border_stroke = browser_row_border_stroke(layout);
        let row_border_rect = browser_row_border_rect(row.rect, row_border_stroke);
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
        if let Some(color) = browser_processing_marker_color(style, row.processing_state) {
            let marker_width = (sizing.border_width * 3.0).clamp(2.0, 5.0);
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: Rect::from_min_max(
                        row.rect.min,
                        Point::new(
                            (row.rect.min.x + marker_width).min(row.rect.max.x),
                            row.rect.max.y,
                        ),
                    ),
                    color,
                }),
            );
        }
        let focus_similarity_reserved_width =
            if row.focused && !model.browser.duplicate_cleanup_active {
                browser_similarity_button_reserved_width(true, sizing)
            } else {
                0.0
            };
        if let Some(marker_rect) =
            browser_playback_age_marker_rect(row.rect, sizing, focus_similarity_reserved_width)
        {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: marker_rect,
                    color: browser_playback_age_marker_color(style, row.playback_age_bucket),
                }),
            );
        }
        push_browser_row_border(
            primitives,
            row_border_rect,
            if row.focused {
                blend_color(
                    style.accent_warning,
                    style.text_primary,
                    style.state_focus_pulse_blend,
                )
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
        if row.locked {
            let focus_left_border_width = if row.focused { row_border_stroke } else { 0.0 };
            if let Some(marker_rect) =
                browser_locked_marker_rect(row.rect, sizing, focus_left_border_width)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect: marker_rect,
                        color: style.accent_mint,
                    }),
                );
            }
        }
        if row.focused {
            let mut label_position = row.text_layout.sample_label.min;
            let show_similarity_button = !model.browser.duplicate_cleanup_active;
            let similarity_button_reserved_width =
                browser_similarity_button_reserved_width(show_similarity_button, sizing);
            let age_marker_reserved_width = browser_playback_age_marker_reserved_width(
                row.rect,
                sizing,
                similarity_button_reserved_width,
            );
            let inline_tag_reserved_width =
                browser_inline_tag_reserved_width_for_labels(&row.inline_tag_labels, sizing);
            let mut label_max_width = row.text_layout.sample_label.width().max(20.0);
            if similarity_button_reserved_width > 0.0 {
                label_position.x = (label_position.x + similarity_button_reserved_width)
                    .min(row.text_layout.sample_label.max.x);
                label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
            }
            if age_marker_reserved_width > 0.0 {
                label_position.x = (label_position.x + age_marker_reserved_width)
                    .min(row.text_layout.sample_label.max.x);
                label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
            }
            label_max_width = (label_max_width
                - browser_rating_indicator_reserved_width(row.rating_level, row.locked, sizing)
                - inline_tag_reserved_width)
                .max(20.0);
            if row.missing {
                let marker_advance =
                    browser_missing_marker_advance(sizing.font_body).min(label_max_width.max(0.0));
                emit_text(
                    text_runs,
                    TextRun {
                        text: String::from(BROWSER_MISSING_CONTENT_MARKER),
                        position: label_position,
                        font_size: sizing.font_body,
                        color: style.accent_trash,
                        max_width: Some(marker_advance),
                        align: TextAlign::Left,
                    },
                );
                label_position.x =
                    (label_position.x + marker_advance).min(row.text_layout.sample_label.max.x);
                label_max_width = (row.text_layout.sample_label.max.x - label_position.x).max(4.0);
            }
            emit_text(
                text_runs,
                TextRun {
                    text: row.visible_row_label.clone(),
                    position: row.text_layout.index_label.min,
                    font_size: sizing.font_meta,
                    color: blend_color(
                        style.accent_warning,
                        style.text_primary,
                        style.state_focus_pulse_blend,
                    ),
                    max_width: Some(row.text_layout.index_label.width().max(12.0)),
                    align: TextAlign::Right,
                },
            );
            if show_similarity_button
                && let Some(button_rect) = browser_similarity_button_rect(row.rect, sizing)
            {
                render_browser_similarity_button(
                    primitives,
                    button_rect,
                    style,
                    sizing,
                    model.browser.similarity_filtered && row.visible_row == 0,
                    blend_color(
                        style.accent_warning,
                        style.text_primary,
                        style.state_focus_pulse_blend,
                    ),
                );
            }
            emit_text(
                text_runs,
                TextRun {
                    text: row.label.clone(),
                    position: label_position,
                    font_size: sizing.font_body,
                    color: blend_color(
                        style.accent_warning,
                        style.text_primary,
                        style.state_focus_pulse_blend,
                    ),
                    max_width: Some(label_max_width),
                    align: TextAlign::Left,
                },
            );
        }
    }
}
