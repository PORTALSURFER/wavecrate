//! Browser panel projection, row-window virtualization, and retained row-cache helpers.

use super::*;

pub(crate) fn project_browser_panel_frame_model(controller: &AppController) -> BrowserPanelModel {
    let selected_visible_row = controller.ui.browser.selected_visible;
    let selected_path_count = controller.ui.browser.selected_paths.len();
    let search_query = controller.ui.browser.search_query.clone();
    let search_placeholder = Some(String::from("Search samples (Ctrl+F)"));
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
        search_placeholder,
        busy,
        sort_label,
        active_tab_label,
        focused_sample_label,
        anchor_visible_row,
        rows: Vec::new(),
    }
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
    let (window_start, window_len) =
        browser_render_window(visible_count, selected_visible_row, anchor_visible_row);
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
                    "SAMPLE",
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

/// Hash a scalar key into one stable 64-bit cache key.
fn hash_scalar<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Hash one relative path into a stable row-identity scalar.
pub(super) fn browser_row_identity_hash(path: &Path) -> u64 {
    hash_scalar(path)
}

/// Clear the retained selected-path lookup cache.
fn clear_projected_selected_paths_lookup(controller: &mut AppController) {
    controller.projected_selected_paths_revision =
        Some(controller.ui.browser.selected_paths_revision);
    controller.projected_selected_paths_lookup = None;
}

/// Refresh the retained selected-index bitset when selection changes.
pub(super) fn refresh_projected_selected_paths_lookup(controller: &mut AppController) {
    let selection_revision = controller.ui.browser.selected_paths_revision;
    if controller.ui.browser.selected_paths.is_empty() {
        if controller.projected_selected_paths_lookup.is_some()
            || controller.projected_selected_paths_revision != Some(selection_revision)
        {
            clear_projected_selected_paths_lookup(controller);
        }
        return;
    }
    if controller.projected_selected_paths_revision == Some(selection_revision)
        && controller.projected_selected_paths_lookup.is_some()
    {
        return;
    }
    let lookup = if controller.ui.browser.selected_paths.len() == 1 {
        controller
            .ui
            .browser
            .selected_paths
            .first()
            .cloned()
            .and_then(|path| controller.wav_index_for_path(path.as_path()))
            .map(ProjectedSelectedPathsLookup::Single)
    } else {
        let mut selected_index_lookup = vec![false; controller.wav_entries_len()];
        for selected_path_idx in 0..controller.ui.browser.selected_paths.len() {
            let selected_path = controller.ui.browser.selected_paths[selected_path_idx].clone();
            if let Some(absolute_index) = controller.wav_index_for_path(selected_path.as_path())
                && let Some(selected) = selected_index_lookup.get_mut(absolute_index)
            {
                *selected = true;
            }
        }
        Some(ProjectedSelectedPathsLookup::Dense(selected_index_lookup))
    };
    controller.projected_selected_paths_revision = Some(selection_revision);
    controller.projected_selected_paths_lookup = lookup;
}

/// Return whether one absolute row index is selected in the retained lookup bitset.
pub(super) fn selected_index_is_selected(
    controller: &AppController,
    absolute_index: usize,
) -> bool {
    match controller.projected_selected_paths_lookup.as_ref() {
        Some(ProjectedSelectedPathsLookup::Single(selected_index)) => {
            *selected_index == absolute_index
        }
        Some(ProjectedSelectedPathsLookup::Dense(lookup)) => {
            lookup.get(absolute_index).copied().unwrap_or(false)
        }
        None => false,
    }
}

/// Clear retained browser-row projection fields.
fn clear_projected_browser_row_cache(controller: &mut AppController) {
    controller.projected_browser_rows.clear();
}

/// Reset retained browser-row projection fields when visible rows changed materially.
pub(super) fn refresh_projected_browser_row_cache(controller: &mut AppController) {
    if controller.projected_browser_rows_revision == controller.ui.browser.visible_rows_revision {
        return;
    }
    controller.projected_browser_rows_revision = controller.ui.browser.visible_rows_revision;
    clear_projected_browser_row_cache(controller);
}

/// Return true when one cached browser-row projection still matches the entry snapshot.
fn cached_browser_row_matches_entry(
    cached: &ProjectedBrowserRowCacheEntry,
    row_identity_hash: u64,
    column_index: usize,
    missing: bool,
) -> bool {
    cached.row_identity_hash == row_identity_hash
        && cached.column_index == column_index
        && cached.missing == missing
}

/// Resolve static browser-row projection fields from cache, inserting on cache miss.
pub(super) fn project_cached_browser_row(
    controller: &mut AppController,
    absolute_index: usize,
) -> Option<(&ProjectedBrowserRowCacheEntry, bool)> {
    let (entry_tag, row_identity_hash, missing) =
        controller.wav_entry(absolute_index).map(|entry| {
            (
                entry.tag,
                browser_row_identity_hash(entry.relative_path.as_path()),
                entry.missing,
            )
        })?;
    let column_index = browser_column_index(entry_tag);
    let cache_hit = controller
        .projected_browser_rows
        .get(&absolute_index)
        .is_some_and(|cached| {
            cached_browser_row_matches_entry(cached, row_identity_hash, column_index, missing)
        });
    trace_browser_row_cache_lookup(cache_hit);
    if !cache_hit {
        let relative_path = controller
            .wav_entry(absolute_index)
            .map(|entry| entry.relative_path.clone())?;
        let row_label = controller
            .label_for_ref(absolute_index)
            .map(str::to_string)
            .unwrap_or_else(|| view_model::sample_display_label(relative_path.as_path()));
        let cached = ProjectedBrowserRowCacheEntry {
            row_identity_hash,
            row_label,
            column_index,
            bucket_label: browser_bucket_label(controller, relative_path.as_path(), entry_tag),
            missing,
        };
        if controller.projected_browser_rows.len() >= MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE {
            clear_projected_browser_row_cache(controller);
        }
        controller
            .projected_browser_rows
            .insert(absolute_index, cached);
    }
    let projected = controller.projected_browser_rows.get(&absolute_index)?;
    Some((
        projected,
        selected_index_is_selected(controller, absolute_index),
    ))
}

/// Write one browser row into `rows[offset]`, reusing existing `String` buffers.
fn write_browser_row_into_slot(
    rows: &mut Vec<BrowserRowModel>,
    offset: usize,
    projection: (usize, &str, usize, &str, bool, bool, bool),
) {
    let (visible_row, row_label, column_index, bucket_label, selected, focused, missing) =
        projection;
    let clamped_column_index = column_index.min(2);
    if let Some(row) = rows.get_mut(offset) {
        if row.visible_row == visible_row && row.column == clamped_column_index {
            row.selected = selected;
            row.focused = focused;
            row.missing = missing;
            if row.label == row_label && row.bucket_label.as_deref() == Some(bucket_label) {
                return;
            }
        }
        row.visible_row = visible_row;
        if row.label != row_label {
            row.label.clear();
            row.label.push_str(row_label);
        }
        row.column = clamped_column_index;
        row.selected = selected;
        row.focused = focused;
        row.missing = missing;
        if let Some(existing_bucket_label) = row.bucket_label.as_mut() {
            if existing_bucket_label != bucket_label {
                existing_bucket_label.clear();
                existing_bucket_label.push_str(bucket_label);
            }
        } else {
            row.bucket_label = Some(bucket_label.to_owned());
        }
        return;
    }
    rows.push(
        BrowserRowModel::new(visible_row, row_label, column_index, selected, focused)
            .with_bucket_label(bucket_label)
            .with_missing(missing),
    );
}

/// Project browser toolbar/tab/footer labels.
pub(crate) fn project_browser_chrome_model(
    ui: &UiState,
    visible_count: usize,
) -> BrowserChromeModel {
    BrowserChromeModel {
        samples_tab_label: String::from("Samples"),
        map_tab_label: String::from("Similarity map"),
        search_prefix_label: String::from("Search"),
        search_placeholder: String::from("Search samples (Ctrl+F)"),
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

pub(super) fn browser_render_window(
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
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
    let half_window = window_len / 2;
    let max_start = visible_count - window_len;
    let window_start = pivot.saturating_sub(half_window).min(max_start);
    (window_start, window_len)
}
