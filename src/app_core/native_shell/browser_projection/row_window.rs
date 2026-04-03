use super::*;
use crate::app_core::app_api::state::BrowserDuplicateCleanupState;

/// Number of rows kept between the focused row and the window edge before scrolling.
///
/// A margin of `3` means the browser starts scrolling once focus reaches the
/// third visible row from the top or bottom, so edge-near selection keeps more
/// look-ahead room during keyboard or pointer navigation.
const BROWSER_RENDER_EDGE_MARGIN_ROWS: usize = 3;

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
        super::clear_projected_selected_paths_lookup(controller);
        rows.clear();
        return;
    }
    super::refresh_projected_browser_row_cache(controller);
    super::refresh_projected_selected_paths_lookup(controller);
    controller.queue_feature_cache_refresh_for_browser();
    let (window_start, window_len) = browser_render_window(
        visible_count,
        selected_visible_row,
        anchor_visible_row,
        controller.ui.browser.selection.autoscroll,
        controller.ui.browser.viewport.render_window_start,
    );
    controller.ui.browser.viewport.render_window_start = window_start;
    super::preload_browser_window_bpms(controller, window_start, window_len);
    let duplicate_cleanup = controller.ui.browser.duplicate_cleanup.clone();
    if rows.capacity() < window_len {
        rows.reserve(window_len.saturating_sub(rows.len()));
    }
    for offset in 0..window_len {
        let visible_row = window_start + offset;
        let Some(absolute_index) = controller.ui.browser.viewport.visible.get(visible_row) else {
            continue;
        };
        let Some((cached_row, selected)) =
            super::project_cached_browser_row(controller, absolute_index)
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
                    PlaybackAgeBucket::Fresh,
                    "",
                    false,
                    focused,
                    false,
                    false,
                    false,
                ),
            );
            continue;
        };
        let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
        let bucket_label = browser_duplicate_cleanup_bucket_label(
            duplicate_cleanup.as_ref(),
            absolute_index,
            &cached_row.bucket_label,
        );
        write_browser_row_into_slot(
            rows,
            offset,
            (
                visible_row,
                &cached_row.row_label,
                cached_row.column_index,
                cached_row.rating_level,
                cached_row.playback_age_bucket,
                &bucket_label,
                selected,
                focused,
                cached_row.missing,
                cached_row.locked,
                cached_row.marked,
            ),
        );
    }
    rows.truncate(window_len);
}

/// Map one sample rating bucket to browser column index.
pub(crate) fn browser_column_index(tag: crate::sample_sources::Rating) -> usize {
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
pub(super) fn browser_bucket_label(
    bpm_value: Option<f32>,
    looped: bool,
    long_sample_mark: bool,
) -> String {
    let mut tags = Vec::new();
    if let Some(bpm) = bpm_value {
        tags.push(format_bpm_badge_label(bpm));
    }
    if looped {
        tags.push(String::from("LOOP"));
    }
    if long_sample_mark {
        tags.push(String::from("LONG"));
    }
    tags.join(" · ")
}

/// Clear retained browser-row projection fields.
pub(super) fn clear_projected_browser_row_cache(controller: &mut AppController) {
    controller.projected_browser_rows.clear();
    controller.projected_browser_preload_window = None;
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

/// Append transient duplicate-cleanup badges to one browser-row metadata label.
fn browser_duplicate_cleanup_bucket_label(
    duplicate_cleanup: Option<&BrowserDuplicateCleanupState>,
    absolute_index: usize,
    base_label: &str,
) -> String {
    let Some(cleanup) = duplicate_cleanup else {
        return base_label.to_owned();
    };
    let mut tags = Vec::new();
    if !base_label.is_empty() {
        tags.push(base_label.to_owned());
    }
    if cleanup.is_anchor(absolute_index) {
        tags.push(String::from("ANCHOR"));
    } else if cleanup.is_kept(absolute_index) {
        tags.push(String::from("KEEP"));
    }
    tags.join(" · ")
}

/// Convert one app-core playback-age bucket into the native radiant contract enum.
fn native_playback_age_bucket(
    bucket: PlaybackAgeBucket,
) -> crate::app_core::actions::NativePlaybackAgeBucket {
    match bucket {
        PlaybackAgeBucket::Fresh => crate::app_core::actions::NativePlaybackAgeBucket::Fresh,
        PlaybackAgeBucket::OlderThanWeek => {
            crate::app_core::actions::NativePlaybackAgeBucket::OlderThanWeek
        }
        PlaybackAgeBucket::OlderThanMonth => {
            crate::app_core::actions::NativePlaybackAgeBucket::OlderThanMonth
        }
        PlaybackAgeBucket::NeverPlayed => {
            crate::app_core::actions::NativePlaybackAgeBucket::NeverPlayed
        }
    }
}

/// Write one browser row into `rows[offset]`, reusing existing `String` buffers.
fn write_browser_row_into_slot(
    rows: &mut Vec<BrowserRowModel>,
    offset: usize,
    projection: (
        usize,
        &str,
        usize,
        i8,
        PlaybackAgeBucket,
        &str,
        bool,
        bool,
        bool,
        bool,
        bool,
    ),
) {
    let (
        visible_row,
        row_label,
        column_index,
        rating_level,
        playback_age_bucket,
        bucket_label,
        selected,
        focused,
        missing,
        locked,
        marked,
    ) = projection;
    let bucket_label = (!bucket_label.is_empty()).then_some(bucket_label);
    let clamped_column_index = column_index.min(2);
    let native_playback_age_bucket = native_playback_age_bucket(playback_age_bucket);
    if let Some(row) = rows.get_mut(offset) {
        if row.visible_row == visible_row && row.column == clamped_column_index {
            row.selected = selected;
            row.focused = focused;
            row.missing = missing;
            row.locked = locked;
            row.marked = marked;
            row.rating_level = rating_level.clamp(-3, 3);
            row.playback_age_bucket = native_playback_age_bucket;
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
        row.playback_age_bucket = native_playback_age_bucket;
        row.selected = selected;
        row.focused = focused;
        row.missing = missing;
        row.locked = locked;
        row.marked = marked;
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
        .with_playback_age_bucket(native_playback_age_bucket)
        .with_missing(missing)
        .with_locked(locked)
        .with_marked(marked);
    if let Some(bucket_label) = bucket_label {
        row = row.with_bucket_label(bucket_label);
    }
    rows.push(row);
}

pub(crate) fn browser_render_window(
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
    autoscroll: bool,
    current_window_start: usize,
) -> (usize, usize) {
    if visible_count == 0 {
        return (0, 0);
    }
    let window_len = visible_count.min(MAX_RENDERED_BROWSER_ROWS);
    if window_len == visible_count {
        return if autoscroll {
            (0, window_len)
        } else {
            (current_window_start.min(visible_count - 1), window_len)
        };
    }
    if !autoscroll {
        return (
            current_window_start.min(visible_count - window_len),
            window_len,
        );
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
