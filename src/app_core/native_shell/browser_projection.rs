//! Browser panel projection, row-window virtualization, and retained row-cache helpers.

use super::*;
use std::path::Path;

/// Retained selection/row cache helpers for browser projection.
mod cache;
use cache::clear_projected_selected_paths_lookup;

#[cfg(test)]
pub(super) use cache::{browser_row_identity_hash, selected_index_is_selected};
pub(super) use cache::{
    project_cached_browser_row, refresh_projected_browser_row_cache,
    refresh_projected_selected_paths_lookup,
};

/// Project browser panel frame metadata without materializing row contents.
///
/// Callers can combine this with row-window projection helpers to refresh
/// metadata and row payloads independently when only one segment is dirty.
pub(crate) fn project_browser_panel_frame_model(controller: &AppController) -> BrowserPanelModel {
    let selected_visible_row = controller.ui.browser.selected_visible;
    let selected_path_count = controller.ui.browser.selected_paths.len();
    let search_query = controller.ui.browser.search_query.clone();
    let active_rating_filters = browser_rating_filter_flags(&controller.ui.browser.rating_filter);
    let search_placeholder = Some(browser_search_placeholder(
        controller.ui.browser.search_focus_requested,
    ));
    let busy = controller.ui.browser.search_busy;
    let sort_label =
        Some(browser_sort_label(SampleBrowserSort::from(controller.ui.browser.sort)).to_owned());
    let active_tab_label = Some(browser_tab_label(controller.ui.browser.active_tab).to_owned());
    let focused_sample_label = controller
        .ui
        .loaded_wav
        .as_deref()
        .map(view_model::sample_display_label);
    let anchor_visible_row = controller.ui.browser.selection_anchor_visible;
    let visible_count = controller.ui.browser.visible.len();
    BrowserPanelModel {
        visible_count,
        selected_visible_row,
        selected_path_count,
        search_query,
        active_rating_filters,
        search_placeholder,
        busy,
        sort_label,
        active_tab_label,
        focused_sample_label,
        anchor_visible_row,
        rows: Vec::new(),
    }
}

/// Project active browser rating-filter levels into a fixed `-3..=3` chip-state array.
fn browser_rating_filter_flags(rating_filter: &std::collections::BTreeSet<i8>) -> [bool; 7] {
    let mut flags = [false; 7];
    for (index, level) in (-3..=3).enumerate() {
        flags[index] = rating_filter.contains(&level);
    }
    flags
}

/// Project browser row content for the current visible window.
///
/// This helper is intentionally separated from metadata projection so callers
/// can refresh row content independently of browser header/search/tab state.
pub(crate) fn project_browser_rows_model(
    controller: &mut AppController,
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
) -> Vec<BrowserRowModel> {
    let mut rows = Vec::with_capacity(visible_count.min(MAX_RENDERED_BROWSER_ROWS));
    project_browser_rows_model_into(
        controller,
        visible_count,
        selected_visible_row,
        anchor_visible_row,
        &mut rows,
    );
    rows
}

/// Project browser row content into an existing row-model buffer.
///
/// Callers that retain `rows` across frames can reuse vector capacity to
/// reduce allocation churn in high-frequency browser projection paths.
pub(crate) fn project_browser_rows_model_into(
    controller: &mut AppController,
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
    rows: &mut Vec<BrowserRowModel>,
) {
    if controller.ui.browser.active_tab == SampleBrowserTab::Map {
        clear_projected_browser_row_cache(controller);
        clear_projected_selected_paths_lookup(controller);
        rows.clear();
        return;
    }
    refresh_projected_browser_row_cache(controller);
    refresh_projected_selected_paths_lookup(controller);
    controller.prepare_feature_cache_for_browser();
    let (window_start, window_len) = browser_render_window(
        visible_count,
        selected_visible_row,
        anchor_visible_row,
        controller.ui.browser.render_window_start,
    );
    controller.ui.browser.render_window_start = window_start;
    preload_browser_window_bpms(controller, window_start, window_len);
    if rows.capacity() < window_len {
        rows.reserve(window_len.saturating_sub(rows.len()));
    }
    for offset in 0..window_len {
        let visible_row = window_start + offset;
        let Some(absolute_index) = controller.ui.browser.visible.get(visible_row) else {
            continue;
        };
        let Some((cached_row, selected)) = project_cached_browser_row(controller, absolute_index)
        else {
            let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
            write_browser_row_into_slot(
                rows,
                offset,
                (
                    visible_row,
                    &format!("row {}", visible_row + 1),
                    1,
                    0,
                    "",
                    false,
                    focused,
                    false,
                ),
            );
            continue;
        };
        let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
        write_browser_row_into_slot(
            rows,
            offset,
            (
                visible_row,
                &cached_row.row_label,
                cached_row.column_index,
                cached_row.rating_level,
                &cached_row.bucket_label,
                selected,
                focused,
                cached_row.missing,
            ),
        );
    }
    rows.truncate(window_len);
}

/// Preload BPM metadata for the current visible browser window in one batch query.
fn preload_browser_window_bpms(
    controller: &mut AppController,
    window_start: usize,
    window_len: usize,
) {
    if window_len == 0 {
        return;
    }
    let mut visible_paths = Vec::with_capacity(window_len);
    for offset in 0..window_len {
        let visible_row = window_start + offset;
        let Some(absolute_index) = controller.ui.browser.visible.get(visible_row) else {
            continue;
        };
        if let Some(relative_path) = controller
            .wav_entry(absolute_index)
            .map(|entry| entry.relative_path.clone())
        {
            visible_paths.push(relative_path);
        }
    }
    controller.preload_bpm_values_for_paths(&visible_paths);
}

/// Project browser panel metadata and row window into one panel model.
pub(crate) fn project_browser_model(controller: &mut AppController) -> BrowserPanelModel {
    let mut panel = project_browser_panel_frame_model(controller);
    panel.rows = project_browser_rows_model(
        controller,
        panel.visible_count,
        panel.selected_visible_row,
        panel.anchor_visible_row,
    );
    panel
}

/// Map one sample rating bucket to browser column index.
pub(super) fn browser_column_index(tag: crate::sample_sources::Rating) -> usize {
    if tag.is_trash() {
        0
    } else if tag.is_keep() {
        2
    } else {
        1
    }
}

/// Resolve one inline browser metadata label for the sample lane.
///
/// Rating text is intentionally omitted because the row already renders keep/trash
/// state via the right-edge indicator rectangles.
fn browser_bucket_label(
    controller: &mut AppController,
    absolute_index: usize,
    relative_path: &Path,
    looped: bool,
) -> String {
    let mut tags = Vec::new();
    if let Some(bpm) = controller.bpm_value_for_path(relative_path) {
        tags.push(format_bpm_badge_label(bpm));
    }
    if looped {
        tags.push(String::from("LOOP"));
    }
    if controller
        .cached_feature_status_for_entry(absolute_index)
        .and_then(|status| status.long_sample_mark)
        == Some(true)
    {
        tags.push(String::from("LONG"));
    }
    tags.join(" · ")
}

/// Format one BPM metadata label for inline browser-row display.
fn format_bpm_badge_label(bpm: f32) -> String {
    if !bpm.is_finite() || bpm <= 0.0 {
        return String::new();
    }
    let rounded = bpm.round();
    if (bpm - rounded).abs() < 0.05 {
        format!("{rounded:.0} BPM")
    } else {
        format!("{bpm:.1} BPM")
    }
}

/// Format one browser sort label for chrome projection.
fn browser_sort_label(sort: SampleBrowserSort) -> &'static str {
    match sort {
        SampleBrowserSort::ListOrder => "List order",
        SampleBrowserSort::Similarity => "Similarity",
        SampleBrowserSort::PlaybackAgeAsc => "Playback age ↑",
        SampleBrowserSort::PlaybackAgeDesc => "Playback age ↓",
    }
}

/// Format one browser tab label for chrome projection.
fn browser_tab_label(tab: SampleBrowserTab) -> &'static str {
    match tab {
        SampleBrowserTab::List => "Samples",
        SampleBrowserTab::Map => "Similarity map",
    }
}

/// Clear retained browser-row projection fields.
fn clear_projected_browser_row_cache(controller: &mut AppController) {
    controller.projected_browser_rows.clear();
}

/// Write one browser row into `rows[offset]`, reusing existing `String` buffers.
fn write_browser_row_into_slot(
    rows: &mut Vec<BrowserRowModel>,
    offset: usize,
    projection: (usize, &str, usize, i8, &str, bool, bool, bool),
) {
    let (
        visible_row,
        row_label,
        column_index,
        rating_level,
        bucket_label,
        selected,
        focused,
        missing,
    ) = projection;
    let bucket_label = (!bucket_label.is_empty()).then_some(bucket_label);
    let clamped_column_index = column_index.min(2);
    if let Some(row) = rows.get_mut(offset) {
        if row.visible_row == visible_row && row.column == clamped_column_index {
            row.selected = selected;
            row.focused = focused;
            row.missing = missing;
            row.rating_level = rating_level.clamp(-3, 3);
            if row.label == row_label && row.bucket_label.as_deref() == bucket_label {
                return;
            }
        }
        row.visible_row = visible_row;
        if row.label != row_label {
            row.label.clear();
            row.label.push_str(row_label);
        }
        row.column = clamped_column_index;
        row.rating_level = rating_level.clamp(-3, 3);
        row.selected = selected;
        row.focused = focused;
        row.missing = missing;
        if let Some(bucket_label) = bucket_label {
            if let Some(existing_bucket_label) = row.bucket_label.as_mut() {
                if existing_bucket_label != bucket_label {
                    existing_bucket_label.clear();
                    existing_bucket_label.push_str(bucket_label);
                }
            } else {
                row.bucket_label = Some(bucket_label.to_owned());
            }
        } else {
            row.bucket_label = None;
        }
        return;
    }
    let mut row = BrowserRowModel::new(visible_row, row_label, column_index, selected, focused)
        .with_rating_level(rating_level)
        .with_missing(missing);
    if let Some(bucket_label) = bucket_label {
        row = row.with_bucket_label(bucket_label);
    }
    rows.push(row);
}

/// Project browser toolbar/tab/footer labels.
pub(crate) fn project_browser_chrome_model(
    ui: &UiState,
    visible_count: usize,
) -> BrowserChromeModel {
    let search_focused = ui.browser.search_focus_requested;
    BrowserChromeModel {
        samples_tab_label: String::from("Samples"),
        map_tab_label: String::from("Similarity map"),
        search_prefix_label: if search_focused {
            String::from("Search • focused")
        } else {
            String::from("Search")
        },
        search_placeholder: browser_search_placeholder(search_focused),
        activity_ready_label: String::from("Ready"),
        activity_busy_label: String::from("Filtering"),
        sort_prefix_label: String::from("Sort"),
        sort_order_label: browser_sort_label(SampleBrowserSort::from(ui.browser.sort)).to_owned(),
        similarity_toggle_label: if ui.browser.similarity_sort_follow_loaded {
            String::from("follow loaded")
        } else {
            String::from("manual anchor")
        },
        item_count_label: format!("{visible_count} items"),
    }
}

/// Resolve search placeholder text, including a focused caret hint when active.
fn browser_search_placeholder(search_focused: bool) -> String {
    if search_focused {
        String::from("▌")
    } else {
        String::from("Search samples (Ctrl+F)")
    }
}

/// Number of rows kept as a guard band before browser focus nudges the visible window.
const BROWSER_RENDER_EDGE_MARGIN_ROWS: usize = 4;

pub(super) fn browser_render_window(
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
    current_window_start: usize,
) -> (usize, usize) {
    if visible_count == 0 {
        return (0, 0);
    }
    let window_len = visible_count.min(MAX_RENDERED_BROWSER_ROWS);
    if window_len == visible_count {
        return (0, window_len);
    }
    let pivot = selected_visible_row
        .or(anchor_visible_row)
        .unwrap_or(0)
        .min(visible_count - 1);
    let max_start = visible_count - window_len;
    let edge_margin = BROWSER_RENDER_EDGE_MARGIN_ROWS.min(window_len.saturating_sub(1) / 2);
    let mut window_start = current_window_start.min(max_start);
    let window_end = window_start + window_len;
    let top_guard = window_start + edge_margin;
    let bottom_guard = window_end.saturating_sub(edge_margin);
    if pivot < top_guard {
        window_start = pivot.saturating_sub(edge_margin);
    } else if pivot >= bottom_guard {
        window_start = pivot
            .saturating_add(edge_margin + 1)
            .saturating_sub(window_len);
    }
    window_start = window_start.min(max_start);
    (window_start, window_len)
}
