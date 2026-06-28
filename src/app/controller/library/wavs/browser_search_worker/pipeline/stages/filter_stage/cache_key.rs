//! Cache-key identity for retained browser-search filter stages.

use super::super::super::*;
use super::playback_age_token::playback_age_filter_cache_token;
use std::hash::{Hash, Hasher};

pub(super) fn filter_stage_required(job: &SearchJob, has_folder_filters: bool) -> bool {
    has_folder_filters
        || job.filter != TriageFlagFilter::All
        || !job.rating_filter.is_empty()
        || !job.playback_age_filter.is_empty()
        || !job.sidebar_filters.is_empty()
        || job.tag_named_filter != crate::app::state::TagNamedFilter::All
}

pub(super) fn filter_stage_hash(
    cache: &mut SearchWorkerCache,
    job: &SearchJob,
    has_folder_filters: bool,
) -> u64 {
    hash_value(&(
        filter_key(job.filter),
        hash_value(&job.rating_filter),
        hash_value(&job.playback_age_filter),
        playback_age_filter_cache_token(
            cache,
            &job.playback_age_filter,
            job.playback_age_now_unix_secs,
        ),
        job.tag_named_filter,
        hash_value(&job.sidebar_filters),
        job.sidebar_filters
            .needs_bpm_metadata()
            .then(|| sidebar_bpm_hash_for_job(job)),
        has_folder_filters.then_some(super::super::super::folder_filter_hash_for_job(job)),
    ))
}

fn sidebar_bpm_hash_for_job(job: &SearchJob) -> u64 {
    hash_value(
        &job.sidebar_bpm_values
            .iter()
            .map(|(path, bpm)| (path, bpm.map(f32::to_bits)))
            .collect::<Vec<_>>(),
    )
}

fn filter_key(filter: TriageFlagFilter) -> u8 {
    match filter {
        TriageFlagFilter::All => 0,
        TriageFlagFilter::Keep => 1,
        TriageFlagFilter::Trash => 2,
        TriageFlagFilter::Untagged => 3,
    }
}

pub(super) fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
