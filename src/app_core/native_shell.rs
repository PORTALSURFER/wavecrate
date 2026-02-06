//! Native-shell projection helpers used by the `radiant` bridge.
//!
//! The bridge consumes these helpers to project controller state into
//! backend-neutral `radiant::app` models and to translate normalized UI ranges
//! back into controller-domain selection math.

use crate::{
    egui_app::{
        controller::EguiController,
        state::{TriageFlagColumn, UiState},
        view_model,
    },
    selection::SelectionRange,
};
use radiant::app::{
    AppModel, BrowserPanelModel, BrowserRowModel, ColumnModel, NormalizedRangeModel,
    SourceRowModel, SourcesPanelModel, WaveformPanelModel,
};
use std::collections::HashSet;

const MAX_RENDERED_BROWSER_ROWS: usize = 48;

pub(crate) fn project_app_model(controller: &mut EguiController) -> AppModel {
    let selected_column = selected_column_index(&controller.ui);
    let transport_running = controller.is_playing();
    let sources = project_sources_model(&controller.ui);
    let status_text = controller.ui.status.text.clone();
    let column_counts = [
        controller.ui.browser.trash.len(),
        controller.ui.browser.neutral.len(),
        controller.ui.browser.keep.len(),
    ];
    let waveform = project_waveform_model(&controller.ui);
    let browser = project_browser_model(controller);
    AppModel {
        title: String::from("Sempal"),
        backend_label: String::from("backend: native_vello"),
        sources_label: format!("Sources ({})", sources.rows.len()),
        status_text,
        columns: [
            ColumnModel::new("Trash", column_counts[0]),
            ColumnModel::new("Samples", column_counts[1]),
            ColumnModel::new("Keep", column_counts[2]),
        ],
        selected_column,
        transport_running,
        sources,
        browser,
        waveform,
    }
}

pub(crate) fn selected_column_index(ui: &UiState) -> usize {
    ui.browser
        .selected
        .map(|selected| match selected.column {
            TriageFlagColumn::Trash => 0,
            TriageFlagColumn::Neutral => 1,
            TriageFlagColumn::Keep => 2,
        })
        .unwrap_or(1)
}

pub(crate) fn browser_focus_target(ui: &UiState, delta: i8) -> Option<usize> {
    let visible_count = ui.browser.visible.len();
    if visible_count == 0 {
        return None;
    }
    let base = ui
        .browser
        .selected_visible
        .unwrap_or(0)
        .min(visible_count - 1);
    Some((base as isize + delta as isize).clamp(0, visible_count as isize - 1) as usize)
}

pub(crate) fn normalized_from_milli(value: u16) -> f32 {
    (value.min(1000) as f32) / 1000.0
}

pub(crate) fn selection_range_from_milli(start_milli: u16, end_milli: u16) -> SelectionRange {
    SelectionRange::new(
        normalized_from_milli(start_milli),
        normalized_from_milli(end_milli),
    )
}

fn project_sources_model(ui: &UiState) -> SourcesPanelModel {
    SourcesPanelModel {
        header: format!("Sources ({})", ui.sources.rows.len()),
        search_query: ui.sources.folders.search_query.clone(),
        selected_row: ui.sources.selected,
        rows: ui
            .sources
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                SourceRowModel::new(
                    row.name.clone(),
                    row.path.clone(),
                    ui.sources
                        .selected
                        .is_some_and(|selected| selected == row_index),
                    row.missing,
                )
            })
            .collect(),
    }
}

fn project_browser_model(controller: &mut EguiController) -> BrowserPanelModel {
    let visible = controller.ui.browser.visible.clone();
    let selected_visible_row = controller.ui.browser.selected_visible;
    let selected_path_count = controller.ui.browser.selected_paths.len();
    let search_query = controller.ui.browser.search_query.clone();
    let busy = controller.ui.browser.search_busy;
    let focused_sample_label = controller
        .ui
        .loaded_wav
        .as_deref()
        .map(view_model::sample_display_label);
    let anchor_visible_row = controller.ui.browser.selection_anchor_visible;
    let selected_paths: HashSet<_> = controller.ui.browser.selected_paths.iter().cloned().collect();

    let mut rows = Vec::new();
    let visible_count = visible.len();
    let rendered = visible_count.min(MAX_RENDERED_BROWSER_ROWS);
    for visible_row in 0..rendered {
        let Some(absolute_index) = visible.get(visible_row) else {
            continue;
        };
        if let Some(entry) = controller.wav_entry(absolute_index) {
            let selected = selected_paths.contains(&entry.relative_path);
            rows.push(BrowserRowModel::new(
                visible_row,
                view_model::sample_display_label(&entry.relative_path),
                browser_column_index(entry.tag),
                selected,
                selected_visible_row.is_some_and(|focused| focused == visible_row),
            ));
        } else {
            rows.push(BrowserRowModel::new(
                visible_row,
                format!("row {}", visible_row + 1),
                1,
                false,
                selected_visible_row.is_some_and(|focused| focused == visible_row),
            ));
        }
    }

    BrowserPanelModel {
        visible_count,
        selected_visible_row,
        selected_path_count,
        search_query,
        busy,
        focused_sample_label,
        anchor_visible_row,
        rows,
    }
}

fn project_waveform_model(ui: &UiState) -> WaveformPanelModel {
    WaveformPanelModel {
        loaded_label: ui
            .loaded_wav
            .as_deref()
            .map(view_model::sample_display_label),
        cursor_milli: ui.waveform.cursor.map(normalized_to_milli),
        playhead_milli: ui
            .waveform
            .playhead
            .visible
            .then_some(normalized_to_milli(ui.waveform.playhead.position)),
        selection_milli: ui.waveform.selection.map(|selection| {
            NormalizedRangeModel::new(
                normalized_to_milli(selection.start()),
                normalized_to_milli(selection.end()),
            )
        }),
        view_start_milli: normalized64_to_milli(ui.waveform.view.start),
        view_end_milli: normalized64_to_milli(ui.waveform.view.end),
        loop_enabled: ui.waveform.loop_enabled,
    }
}

fn browser_column_index(tag: crate::sample_sources::Rating) -> usize {
    if tag.is_trash() {
        0
    } else if tag.is_keep() {
        2
    } else {
        1
    }
}

fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn normalized64_to_milli(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_column_defaults_to_middle_column_without_selection() {
        let ui = UiState::default();
        assert_eq!(selected_column_index(&ui), 1);
    }

    #[test]
    fn normalized_from_milli_clamps_bounds() {
        assert_eq!(normalized_from_milli(0), 0.0);
        assert_eq!(normalized_from_milli(455), 0.455);
        assert_eq!(normalized_from_milli(2000), 1.0);
    }

    #[test]
    fn browser_focus_target_clamps_to_visible_window() {
        let mut ui = UiState::default();
        ui.browser.visible = crate::egui_app::state::VisibleRows::List(vec![0, 1, 2, 3]);
        ui.browser.selected_visible = Some(1);

        assert_eq!(browser_focus_target(&ui, -8), Some(0));
        assert_eq!(browser_focus_target(&ui, 1), Some(2));
        assert_eq!(browser_focus_target(&ui, 99), Some(3));
    }

    #[test]
    fn browser_column_index_maps_rating_buckets() {
        assert_eq!(
            browser_column_index(crate::sample_sources::Rating::TRASH_1),
            0
        );
        assert_eq!(
            browser_column_index(crate::sample_sources::Rating::NEUTRAL),
            1
        );
        assert_eq!(
            browser_column_index(crate::sample_sources::Rating::KEEP_1),
            2
        );
    }

    #[test]
    fn selection_range_from_milli_clamps_and_orders_bounds() {
        let range = selection_range_from_milli(750, 250);
        assert_eq!(range.start(), 0.25);
        assert_eq!(range.end(), 0.75);

        let range = selection_range_from_milli(2000, 0);
        assert_eq!(range.start(), 0.0);
        assert_eq!(range.end(), 1.0);
    }
}
