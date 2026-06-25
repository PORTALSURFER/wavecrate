use super::base_stage::ensure_base_stage;
use super::folder_stage::ensure_folder_acceptance_stage;
use super::*;
use crate::app::controller::test_support::prepare_with_source_and_wav_entries;
use crate::app::state::{
    BrowserDuplicateCleanupState, FolderFileScopeMode, PlaybackAgeFilterChip, SampleBrowserSort,
    SimilarQuery, TriageFlagFilter, VisibleRows,
};
use crate::sample_sources::Rating;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

fn playback_age_cache_token(
    controller: &AppController,
    filters: &BTreeSet<PlaybackAgeFilterChip>,
) -> Option<PlaybackAgeTokenCache> {
    let base_fingerprint_hash =
        super::helpers::hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    let filter_hash = super::helpers::hash_value(filters);
    controller
        .ui_cache
        .browser
        .pipeline
        .playback_age_token_caches
        .iter()
        .copied()
        .find(|cached| {
            cached.base_fingerprint_hash == base_fingerprint_hash
                && cached.filter_hash == filter_hash
        })
}

mod base_partition;
mod cleanup_filters;
mod compact_query;
mod filter_stages;
mod metadata_updates;
mod playback_age_cache;
mod similarity_order;

fn search_entry(
    path: &str,
    tag: Rating,
    last_played_at: Option<i64>,
) -> crate::sample_sources::WavEntry {
    crate::sample_sources::WavEntry {
        relative_path: PathBuf::from(path),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag,
        looped: false,
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at,
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }
}

fn clear_loaded_wav_pages(controller: &mut crate::app::controller::AppController) {
    controller.wav_entries.pages.clear();
    controller.wav_entries.lookup.clear();
    controller.ui_cache.browser.pipeline.invalidate();
    controller.ui_cache.browser.search.invalidate();
}
