use super::super::*;

use crate::app::state::SampleBrowserSort;

/// Rebuild the visible-row stage when only non-query filters are active.
pub(super) fn ensure_sorted_stage_for_filter_only(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
) {
    let sorted_fingerprint =
        helpers::hash_value(&(filtered_fingerprint, helpers::sort_key(sort_mode)));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint == Some(sorted_fingerprint) {
        return;
    }

    let mut visible = controller.ui_cache.browser.pipeline.filtered_rows.clone();
    maybe_sort_visible_by_playback_age(controller, &mut visible, sort_mode);
    controller
        .ui_cache
        .browser
        .pipeline
        .rebuild_sorted_row_positions(&visible);
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}

pub(super) fn maybe_sort_visible_by_playback_age(
    controller: &mut AppController,
    visible: &mut Vec<usize>,
    sort_mode: SampleBrowserSort,
) {
    if matches!(
        sort_mode,
        SampleBrowserSort::PlaybackAgeAsc | SampleBrowserSort::PlaybackAgeDesc
    ) {
        helpers::sort_visible_by_playback_age(
            controller,
            visible,
            sort_mode == SampleBrowserSort::PlaybackAgeAsc,
        );
    }
}
