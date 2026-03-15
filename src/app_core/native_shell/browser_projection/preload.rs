use super::*;

/// Preload BPM metadata for the current visible browser window in one batch query.
pub(super) fn preload_browser_window_bpms(
    controller: &mut AppController,
    window_start: usize,
    window_len: usize,
) {
    let source_id = controller.selected_source_id();
    let visible_rows_revision = controller.ui.browser.viewport.visible_rows_revision;
    if window_len == 0 || source_id.is_none() {
        controller.projected_browser_preload_window = Some(ProjectedBrowserPreloadWindow {
            source_id,
            visible_rows_revision,
            window_start,
            window_len,
        });
        return;
    }
    let preload_ranges = browser_bpm_preload_ranges(
        controller.projected_browser_preload_window.as_ref(),
        source_id.as_ref(),
        visible_rows_revision,
        window_start,
        window_len,
    );
    let preload_capacity = preload_ranges.iter().map(|(_, len)| *len).sum();
    let mut visible_paths = Vec::with_capacity(preload_capacity);
    for (range_start, range_len) in preload_ranges {
        append_browser_window_preload_paths(controller, range_start, range_len, &mut visible_paths);
    }
    controller.preload_bpm_values_for_paths(&visible_paths);
    controller.projected_browser_preload_window = Some(ProjectedBrowserPreloadWindow {
        source_id,
        visible_rows_revision,
        window_start,
        window_len,
    });
}

/// Return the visible-row ranges that newly entered the browser window.
#[cfg(test)]
pub(crate) fn browser_bpm_preload_ranges(
    previous: Option<&ProjectedBrowserPreloadWindow>,
    source_id: Option<&crate::sample_sources::SourceId>,
    visible_rows_revision: u64,
    window_start: usize,
    window_len: usize,
) -> Vec<(usize, usize)> {
    browser_bpm_preload_ranges_impl(
        previous,
        source_id,
        visible_rows_revision,
        window_start,
        window_len,
    )
}

#[cfg(not(test))]
fn browser_bpm_preload_ranges(
    previous: Option<&ProjectedBrowserPreloadWindow>,
    source_id: Option<&crate::sample_sources::SourceId>,
    visible_rows_revision: u64,
    window_start: usize,
    window_len: usize,
) -> Vec<(usize, usize)> {
    browser_bpm_preload_ranges_impl(
        previous,
        source_id,
        visible_rows_revision,
        window_start,
        window_len,
    )
}

fn browser_bpm_preload_ranges_impl(
    previous: Option<&ProjectedBrowserPreloadWindow>,
    source_id: Option<&crate::sample_sources::SourceId>,
    visible_rows_revision: u64,
    window_start: usize,
    window_len: usize,
) -> Vec<(usize, usize)> {
    if window_len == 0 {
        return Vec::new();
    }
    let Some(previous) = previous else {
        return vec![(window_start, window_len)];
    };
    if previous.source_id.as_ref() != source_id
        || previous.visible_rows_revision != visible_rows_revision
    {
        return vec![(window_start, window_len)];
    }
    let window_end = window_start.saturating_add(window_len);
    let previous_end = previous.window_start.saturating_add(previous.window_len);
    if window_end <= previous.window_start || previous_end <= window_start {
        return vec![(window_start, window_len)];
    }
    let mut ranges = Vec::with_capacity(2);
    if window_start < previous.window_start {
        ranges.push((window_start, previous.window_start - window_start));
    }
    if previous_end < window_end {
        ranges.push((previous_end, window_end - previous_end));
    }
    ranges
}

/// Append relative paths for one visible browser window range into the preload buffer.
fn append_browser_window_preload_paths(
    controller: &mut AppController,
    window_start: usize,
    window_len: usize,
    visible_paths: &mut Vec<std::path::PathBuf>,
) {
    for offset in 0..window_len {
        let visible_row = window_start + offset;
        let Some(absolute_index) = controller.ui.browser.viewport.visible.get(visible_row) else {
            continue;
        };
        if let Some(relative_path) = controller
            .wav_entry(absolute_index)
            .map(|entry| entry.relative_path.clone())
        {
            visible_paths.push(relative_path);
        }
    }
}
