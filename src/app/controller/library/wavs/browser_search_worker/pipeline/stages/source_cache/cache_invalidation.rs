use super::super::super::*;

pub(super) fn reset_entries_for_db_reopen(cache: &mut SearchWorkerCache) {
    cache.entries = None;
    cache.entry_lookup.clear();
    cache.revision = 0;
    cache.paths_revision = 0;
}

pub(super) fn clear_source_identity_caches(cache: &mut SearchWorkerCache) {
    cache.path_fingerprint = 0;
    cache.query_score_cache.clear();
}

pub(super) fn clear_path_dependent_scores_if_changed(
    cache: &mut SearchWorkerCache,
    path_fingerprint: u64,
) {
    if cache.path_fingerprint != path_fingerprint {
        cache.path_fingerprint = path_fingerprint;
        cache.query_score_cache.clear();
    }
}

pub(super) fn clear_derived_search_caches(cache: &mut SearchWorkerCache) {
    cache.folder_accept_cache.clear();
    cache.filter_stage_cache.clear();
    cache.playback_age_token_caches.clear();
    cache.triage_cache = None;
}

pub(super) fn clear_metadata_dependent_caches(cache: &mut SearchWorkerCache) {
    cache.filter_stage_cache.clear();
    cache.playback_age_token_caches.clear();
    cache.triage_cache = None;
}
