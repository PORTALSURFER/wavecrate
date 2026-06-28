use super::{
    clear_projected_selected_paths_lookup, preload_browser_window_bpms, project_cached_browser_row,
    refresh_projected_browser_row_cache, refresh_projected_selected_paths_lookup,
    selected_index_is_selected,
};
use crate::app_core::actions::NativeBrowserRowProcessingState as BrowserRowProcessingState;
use crate::app_core::actions::{
    NativeBrowserRowModel as BrowserRowModel, NativeRetainedVec as RetainedVec,
};
use crate::app_core::controller::AppController;
use crate::app_core::controller::AutoRenameBatchRowState;
use crate::app_core::state::{PlaybackAgeBucket, SampleBrowserTab};
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

mod labels;
mod row_model;
mod windowing;

pub(super) use labels::browser_bucket_label;
use labels::browser_duplicate_cleanup_bucket_label;
use row_model::{BrowserRowFlags, BrowserRowProjection, write_browser_row_into_slot};
pub(crate) use windowing::browser_render_window;

/// Project browser row content for the current visible window.
///
/// This helper is intentionally separated from metadata projection so callers
/// can refresh row content independently of browser header/search/tab state.
pub(crate) fn project_browser_rows_model(
    controller: &mut AppController,
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
) -> RetainedVec<BrowserRowModel> {
    let mut rows = RetainedVec::from(Vec::with_capacity(
        visible_count.min(MAX_RENDERED_BROWSER_ROWS),
    ));
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
    rows: &mut RetainedVec<BrowserRowModel>,
) {
    if controller.ui.browser.active_tab == SampleBrowserTab::Map {
        clear_projected_browser_row_cache(controller);
        clear_projected_selected_paths_lookup(controller);
        rows.clear();
        return;
    }
    refresh_projected_browser_row_cache(controller);
    refresh_projected_selected_paths_lookup(controller);
    let (window_start, window_len) = browser_render_window(
        visible_count,
        selected_visible_row,
        anchor_visible_row,
        controller.ui.browser.selection.autoscroll,
        controller.ui.browser.viewport.render_window_start,
    );
    controller.ui.browser.viewport.render_window_start = window_start;
    preload_browser_window_bpms(controller, window_start, window_len);
    let playback_age_now_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let duplicate_cleanup = controller.ui.browser.duplicate_cleanup.clone();
    let similar_query = duplicate_cleanup
        .is_none()
        .then(|| controller.ui.browser.search.similar_query.clone())
        .flatten();
    let processing_states = browser_auto_rename_processing_states(controller);
    if rows.make_mut().capacity() < window_len {
        let additional = window_len.saturating_sub(rows.len());
        rows.make_mut().reserve(additional);
    }
    for offset in 0..window_len {
        let visible_row = window_start + offset;
        let Some(absolute_index) = controller.ui.browser.viewport.visible.get(visible_row) else {
            continue;
        };
        let Some((cached_row, selected)) =
            project_cached_browser_row(controller, absolute_index, playback_age_now_unix_secs)
        else {
            let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
            write_browser_row_into_slot(
                rows,
                offset,
                BrowserRowProjection {
                    visible_row,
                    row_label: &format!("row {}", visible_row + 1),
                    column_index: 1,
                    rating_level: 0,
                    playback_age_bucket: PlaybackAgeBucket::Fresh,
                    bucket_label: "",
                    flags: BrowserRowFlags {
                        selected: false,
                        focused,
                        missing: false,
                        locked: false,
                    },
                    processing_state: BrowserRowProcessingState::None,
                    similarity_display_strength: None,
                },
            );
            continue;
        };
        let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
        let bucket_label = browser_duplicate_cleanup_bucket_label(
            duplicate_cleanup.as_ref(),
            absolute_index,
            &cached_row.bucket_label,
        );
        let similarity_display_strength = similar_query
            .as_ref()
            .and_then(|query| query.display_strength_for_index(absolute_index))
            .map(BrowserRowModel::encode_similarity_display_strength);
        let processing_state = processing_states
            .as_ref()
            .and_then(|states| states.get(cached_row.relative_path.as_path()).copied())
            .unwrap_or(BrowserRowProcessingState::None);
        write_browser_row_into_slot(
            rows,
            offset,
            BrowserRowProjection {
                visible_row,
                row_label: &cached_row.row_label,
                column_index: cached_row.column_index,
                rating_level: cached_row.rating_level,
                playback_age_bucket: cached_row.playback_age_bucket,
                bucket_label: &bucket_label,
                flags: BrowserRowFlags {
                    selected,
                    focused,
                    missing: cached_row.missing,
                    locked: cached_row.locked,
                },
                processing_state,
                similarity_display_strength,
            },
        );
    }
    rows.truncate(window_len);
}

/// Patch row selection and focus flags without rebuilding the full row window.
///
/// Focus and multi-selection changes are much cheaper than row-content changes,
/// so retained projection reuses the current row vector and only rewrites the
/// transient flags that drive highlight styling.
pub(crate) fn patch_browser_rows_state(
    controller: &mut AppController,
    selected_visible_row: Option<usize>,
    rows: &mut [BrowserRowModel],
) {
    refresh_projected_selected_paths_lookup(controller);
    let processing_states = browser_auto_rename_processing_states(controller);
    for row in rows {
        let absolute_index = controller.ui.browser.viewport.visible.get(row.visible_row);
        row.focused = selected_visible_row.is_some_and(|focused| focused == row.visible_row);
        row.selected =
            absolute_index.is_some_and(|index| selected_index_is_selected(controller, index));
        row.processing_state = absolute_index
            .and_then(|index| controller.browser_projection_entry(index))
            .and_then(|entry| {
                processing_states
                    .as_ref()
                    .and_then(|states| states.get(entry.relative_path).copied())
            })
            .unwrap_or(BrowserRowProcessingState::None);
    }
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

/// Clear retained browser-row projection fields.
pub(super) fn clear_projected_browser_row_cache(controller: &mut AppController) {
    controller.projected_browser_rows.clear();
    controller.projected_browser_row_cache_clock = 0;
    controller.projected_browser_preload_window = None;
}

fn browser_auto_rename_processing_states(
    controller: &AppController,
) -> Option<HashMap<std::path::PathBuf, BrowserRowProcessingState>> {
    let selected_source_id = controller.selected_source_id()?;
    let snapshot = controller.active_auto_rename_batch_snapshot_for_projection()?;
    if snapshot.source_id != selected_source_id {
        return None;
    }
    let mut states = HashMap::new();
    for row in snapshot.rows {
        let processing_state = match row.state {
            AutoRenameBatchRowState::Queued => BrowserRowProcessingState::Queued,
            AutoRenameBatchRowState::Active => BrowserRowProcessingState::Active,
            AutoRenameBatchRowState::Completed => BrowserRowProcessingState::Completed,
            AutoRenameBatchRowState::Skipped => BrowserRowProcessingState::Skipped,
            AutoRenameBatchRowState::Failed => BrowserRowProcessingState::Failed,
        };
        states.insert(row.requested_path, processing_state);
        states.insert(row.current_path, processing_state);
    }
    Some(states)
}
