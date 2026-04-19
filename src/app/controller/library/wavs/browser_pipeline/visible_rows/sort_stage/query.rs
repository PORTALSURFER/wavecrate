use super::super::*;
use super::filter_only::maybe_sort_visible_by_playback_age;

use crate::app::state::SampleBrowserSort;

/// Rebuild fuzzy-query scores and visible rows for the active text query.
pub(super) fn ensure_sorted_stage_for_query(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
    query: &str,
) {
    controller.ensure_search_scores(query);
    let score_fingerprint = helpers::hash_value(&(
        filtered_fingerprint,
        helpers::hash_value(query),
        controller.ui_cache.browser.search.scores.len(),
    ));
    if controller.ui_cache.browser.pipeline.scored_fingerprint != Some(score_fingerprint) {
        let filtered_len = controller.ui_cache.browser.pipeline.filtered_rows.len();
        let mut scored = Vec::with_capacity(filtered_len);
        for row in 0..filtered_len {
            let index = controller.ui_cache.browser.pipeline.filtered_rows[row];
            if let Some(score) = controller
                .ui_cache
                .browser
                .search
                .scores
                .get(index)
                .and_then(|score| *score)
            {
                scored.push((index, score));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        controller.ui_cache.browser.pipeline.scored_rows = scored;
        controller.ui_cache.browser.pipeline.scored_fingerprint = Some(score_fingerprint);
        controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
    }

    let sorted_fingerprint = helpers::hash_value(&(
        controller.ui_cache.browser.pipeline.scored_fingerprint,
        helpers::sort_key(sort_mode),
    ));
    if controller.ui_cache.browser.pipeline.sorted_fingerprint == Some(sorted_fingerprint) {
        return;
    }

    let mut visible: Vec<usize> = controller
        .ui_cache
        .browser
        .pipeline
        .scored_rows
        .iter()
        .map(|(index, _)| *index)
        .collect();
    maybe_sort_visible_by_playback_age(controller, &mut visible, sort_mode);
    controller
        .ui_cache
        .browser
        .pipeline
        .rebuild_sorted_row_positions(&visible);
    controller.ui_cache.browser.pipeline.sorted_rows = visible.into();
    controller.ui_cache.browser.pipeline.sorted_fingerprint = Some(sorted_fingerprint);
}
