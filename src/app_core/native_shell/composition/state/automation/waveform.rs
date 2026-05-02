//! Waveform automation snapshot builders.

use super::helpers::{action_slug, bounds, metadata, node_id, selection_micros_text, simple_node};
use super::*;
use crate::compat_app_contract::{AutomationRole, NormalizedRangeModel};

/// Build semantic automation for the waveform panel.
pub(super) fn build_waveform_automation(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let motion_model = NativeMotionModel::from_app_model(model);
    let mut children = Vec::new();
    for button in waveform_toolbar_buttons(
        layout,
        style,
        &motion_model,
        shell.waveform_bpm_input_active,
        shell.waveform_bpm_input_display.as_deref(),
    ) {
        children.push(simple_node(
            format!("waveform.toolbar.{}", super::helpers::slug(button.label)),
            if button.label == "BPM Value" {
                AutomationRole::SearchField
            } else {
                AutomationRole::Button
            },
            Some(String::from(button.label)),
            button.rect,
            button.display_text.clone(),
            button.enabled,
            button.active,
            button
                .action
                .as_ref()
                .map(|action| vec![action_slug(action)])
                .unwrap_or_default(),
        ));
    }
    children.push(AutomationNodeSnapshot {
        id: node_id("waveform.region"),
        role: AutomationRole::TimelineRegion,
        label: Some(String::from("Waveform")),
        bounds: bounds(layout.waveform_plot),
        value: model.waveform.loaded_label.clone(),
        enabled: true,
        selected: matches!(
            model.focus_context,
            crate::compat_app_contract::FocusContextModel::Timeline
        ),
        available_actions: vec![
            String::from("detect_waveform_silence_slices"),
            String::from("detect_waveform_exact_duplicate_slices"),
            String::from("clean_waveform_exact_duplicate_slices"),
            String::from("audition_waveform_duplicate_slice"),
            String::from("toggle_waveform_duplicate_slice_exemption"),
            String::from("move_waveform_slice_focus"),
            String::from("toggle_focused_waveform_slice_export_mark"),
            String::from("play_waveform_at_precise"),
            String::from("clear_waveform_selections"),
            String::from("seek_waveform"),
            String::from("set_waveform_cursor"),
            String::from("set_waveform_selection_range"),
            String::from("zoom_waveform"),
            String::from("set_waveform_view_center"),
        ],
        metadata: metadata(&[
            (
                "loop_enabled",
                super::helpers::bool_text(model.waveform.loop_enabled),
            ),
            (
                "tempo_label",
                model.waveform.tempo_label.as_deref().unwrap_or(""),
            ),
            (
                "zoom_label",
                model.waveform.zoom_label.as_deref().unwrap_or(""),
            ),
            (
                "cursor_milli",
                &model
                    .waveform
                    .cursor_milli
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "selection_micros",
                &model
                    .waveform
                    .selection_milli
                    .map(selection_micros_text)
                    .unwrap_or_default(),
            ),
            (
                "edit_selection_micros",
                &model
                    .waveform
                    .edit_selection_milli
                    .map(selection_micros_text)
                    .unwrap_or_default(),
            ),
            (
                "view_micros",
                &format!(
                    "{}-{}",
                    model.waveform.view_start_micros, model.waveform.view_end_micros
                ),
            ),
        ]),
        children: Vec::new(),
    });
    if let Some(selection) = model.waveform.selection_milli {
        children.push(waveform_selection_node(
            "waveform.selection",
            "Playback selection",
            layout.waveform_plot,
            model,
            selection,
            "clear_waveform_selection",
        ));
    }
    if let Some(selection) = model.waveform.edit_selection_milli {
        children.push(waveform_selection_node(
            "waveform.edit_selection",
            "Edit selection",
            layout.waveform_plot,
            model,
            selection,
            "clear_waveform_edit_selection",
        ));
    }
    children.extend(waveform_slice_nodes(layout.waveform_plot, model));
    AutomationNodeSnapshot {
        id: node_id("waveform.panel"),
        role: AutomationRole::Panel,
        label: Some(String::from("Waveform panel")),
        bounds: bounds(layout.waveform_card),
        value: model.waveform.loaded_label.clone(),
        enabled: true,
        selected: matches!(
            model.focus_context,
            crate::compat_app_contract::FocusContextModel::Timeline
        ),
        available_actions: vec![String::from("focus_waveform_panel")],
        metadata: std::collections::BTreeMap::new(),
        children,
    }
}

fn waveform_slice_nodes(plot: Rect, model: &AppModel) -> Vec<AutomationNodeSnapshot> {
    model
        .waveform
        .slices
        .iter()
        .enumerate()
        .map(|(index, slice)| waveform_slice_node(index, plot, model, slice.clone()))
        .collect()
}

fn waveform_slice_node(
    index: usize,
    plot: Rect,
    model: &AppModel,
    slice: crate::gui::visualization::TimelineMarkerPreview,
) -> AutomationNodeSnapshot {
    let selection_value = selection_micros_text(slice.range);
    AutomationNodeSnapshot {
        id: node_id(format!("waveform.slice.{index:03}")),
        role: AutomationRole::Button,
        label: Some(format!("Slice {}", index + 1)),
        bounds: bounds(waveform_selection_bounds(plot, model, slice.range)),
        value: Some(selection_value.clone()),
        enabled: true,
        selected: slice.selected
            || slice.focused
            || slice.marked_for_export
            || slice.duplicate_cleanup_exempted,
        available_actions: if slice.duplicate_cleanup_candidate {
            vec![
                String::from("audition_waveform_duplicate_slice"),
                String::from("toggle_waveform_duplicate_slice_exemption"),
            ]
        } else {
            vec![String::from("toggle_waveform_slice_selection")]
        },
        metadata: metadata(&[
            ("selection_micros", selection_value.as_str()),
            ("focused", super::helpers::bool_text(slice.focused)),
            (
                "marked_for_export",
                super::helpers::bool_text(slice.marked_for_export),
            ),
            ("edit_selected", super::helpers::bool_text(slice.selected)),
            (
                "duplicate_cleanup_candidate",
                super::helpers::bool_text(slice.duplicate_cleanup_candidate),
            ),
            (
                "duplicate_cleanup_exempted",
                super::helpers::bool_text(slice.duplicate_cleanup_exempted),
            ),
        ]),
        children: Vec::new(),
    }
}

fn waveform_selection_node(
    id: &'static str,
    label: &'static str,
    plot: Rect,
    model: &AppModel,
    range: NormalizedRangeModel,
    clear_action: &'static str,
) -> AutomationNodeSnapshot {
    let selection_value = selection_micros_text(range);
    AutomationNodeSnapshot {
        id: node_id(id),
        role: AutomationRole::Group,
        label: Some(String::from(label)),
        bounds: bounds(waveform_selection_bounds(plot, model, range)),
        value: Some(selection_value.clone()),
        enabled: true,
        selected: true,
        available_actions: vec![String::from(clear_action)],
        metadata: metadata(&[("selection_micros", selection_value.as_str())]),
        children: Vec::new(),
    }
}

fn waveform_selection_bounds(plot: Rect, model: &AppModel, range: NormalizedRangeModel) -> Rect {
    let start_x = waveform_selection_x_for_micros(plot, model, range.start_micros);
    let end_x = waveform_selection_x_for_micros(plot, model, range.end_micros);
    let min_x = start_x.min(end_x);
    let max_x = end_x.max(min_x + 1.0);
    Rect::from_min_max(Point::new(min_x, plot.min.y), Point::new(max_x, plot.max.y))
}

fn waveform_selection_x_for_micros(plot: Rect, model: &AppModel, micros: u32) -> f32 {
    let view = waveform_view_window_from_bounds(
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
        Some(model.waveform.view_start_nanos),
        Some(model.waveform.view_end_nanos),
    );
    waveform_plot_x_for_micros(plot, micros, view, NormalizedPixelSnap::Nearest)
}
