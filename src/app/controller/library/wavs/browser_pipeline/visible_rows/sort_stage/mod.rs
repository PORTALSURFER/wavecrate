mod filter_only;
mod query;
mod similarity;

use super::super::AppController;

use crate::app::state::{SampleBrowserSort, SimilarQuery};

/// Route filter-only visible-row builds through the focused filter-only stage.
pub(super) fn ensure_sorted_stage_for_filter_only(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
) {
    filter_only::ensure_sorted_stage_for_filter_only(controller, filtered_fingerprint, sort_mode);
}

/// Route text-query visible-row builds through the focused query-sort stage.
pub(super) fn ensure_sorted_stage_for_query(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
    query: &str,
) {
    query::ensure_sorted_stage_for_query(controller, filtered_fingerprint, sort_mode, query);
}

/// Route similarity visible-row builds through the focused similarity stage.
pub(super) fn ensure_sorted_stage_for_similar(
    controller: &mut AppController,
    filtered_fingerprint: u64,
    sort_mode: SampleBrowserSort,
    similar: &SimilarQuery,
    playback_age_now_unix_secs: i64,
) {
    similarity::ensure_sorted_stage_for_similar(
        controller,
        filtered_fingerprint,
        sort_mode,
        similar,
        playback_age_now_unix_secs,
    );
}
